/*
Copyright 2021-2023 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/
//! Helper functions for figuring out how to configure the `cargo` execution environment
use std::env::VarError;
use std::ffi::CStr;
use std::path::Path;
use std::process::Command;

use pgrx::{pg_sys, PgMemoryContexts};

use crate::gucs::PLRUST_PATH_OVERRIDE;
use crate::target::CrossCompilationTarget;

/// Builds a `Command::new("cargo")` with necessary environment variables pre-configured
pub(crate) fn cargo(
    cargo_target_dir: &Path,
    cross_compilation_target: Option<CrossCompilationTarget>,
) -> eyre::Result<Command> {
    let mut command = Command::new("cargo");

    configure_path(&mut command)?;
    configure_rustc(&mut command);
    configure_pg_config(&mut command, cross_compilation_target);
    sanitize_env(&mut command);

    command.env("CARGO_TARGET_DIR", &cargo_target_dir);
    if cfg!(target_os = "macos") {
        command.env("RUSTFLAGS", "-Clink-args=-Wl,-undefined,dynamic_lookup");
    } else {
        // Don't use `env_remove` to avoid inheriting rustflags via the normal
        // search.
        command.env("RUSTFLAGS", "");
    }

    Ok(command)
}

/// `cargo` needs a PATH in order to find its tools and we have some rules about setting that up...
///
/// If the `plrust.PATH_override` GUC is set, we just blindly use it.  Otherwise, if PATH is set,
/// we'll use that.  Otherwise we'll create one of `~/.cargo/bin:/usr/bin` and hope it's good enough.
fn configure_path(command: &mut Command) -> eyre::Result<()> {
    if let Some(path) = PLRUST_PATH_OVERRIDE.get() {
        // we were configured with an explicit $PATH to use
        command.env("PATH", path);
    } else {
        let is_empty = match std::env::var("PATH") {
            Ok(s) if s.trim().is_empty() => true,
            Ok(_) => false,
            Err(VarError::NotPresent) => true,
            Err(e) => return Err(eyre::eyre!(e)),
        };

        if is_empty {
            // the environment has no $PATH, so lets try and make a good one based on where
            // we'd expect 'cargo' to be installed
            if let Ok(path) = home::cargo_home() {
                let path = path.join("bin");
                command.env(
                    "PATH",
                    std::env::join_paths(vec![path.as_path(), std::path::Path::new("/usr/bin")])?,
                );
            } else {
                // we don't have a home directory... where could cargo be?  Ubuntu installed cargo
                // at least puts it in /usr/bin
                command.env("PATH", "/usr/bin");
            }
        }
    }
    Ok(())
}

/// PL/Rust uses is own rustc driver named `plrustc`, and it is expected that it be on the path.
/// We use our own rustc driver so we can enable our various lints
fn configure_rustc(command: &mut Command) {
    // TODO:  Do we want a GUC to set this if maybe `which plrustc` fails?
    command.env("RUSTC", "plrustc");
}

/// There was a time in the past where plrust had a `plrust.pg_config` GUC whose value was
/// passed down to the "pgrx-pg-sys" transient dependency via an environment variable.
///
/// This turned out to be an unwanted bit of user, system, and operational complexity.
///
/// Instead, we tell the environment that "pg_config" is described as environment
/// variables, and set every property Postgres can tell us (which is essentially how
/// `pg_config` itself works) as individual environment variables, each prefixed with
/// "PGRX_PG_CONFIG_".
///
/// "pgrx-pg-sys"'s build.rs knows how to interpret these environment variables to get what
/// it needs to properly generate bindings.
fn configure_pg_config(
    command: &mut Command,
    cross_compilation_target: Option<CrossCompilationTarget>,
) {
    command.env("PGRX_PG_CONFIG_AS_ENV", "true");
    for (k, v) in pg_config_values() {
        let k = format!("PGRX_PG_CONFIG_{k}");
        command.env(k, v);
    }

    // set environment variables we need in order for a cross compile
    if let Some(target_triple) = cross_compilation_target {
        // the CARGO_TARGET_xx_LINKER variable
        let (k, v) = target_triple.linker_envar();
        command.env(k, v);

        // pgrx-specified variable for where the bindings are
        if let Some((k, v)) = target_triple.bindings_envar() {
            command.env(k, v);
        }
    }
}

/// these are environment variables that could possibly impact pgrx compilation that we don't
/// want to accidentally or purposely be inherited from the running "postgres" process
fn sanitize_env(command: &mut Command) {
    command.env_remove("DOCS_RS"); // we'll never be building user function on https://docs.rs
    command.env_remove("PGRX_BUILD_VERBOSE"); // unnecessary to ever build a user function in verbose mode
    command.env_remove("PGRX_PG_SYS_GENERATE_BINDINGS_FOR_RELEASE"); // while an interesting idea, PL/Rust user functions are not used to generate a `pgrx` release
    command.env_remove("CARGO_MANIFEST_DIR"); // we are in the manifest directory b/c of `command.current_dir()` above
    command.env_remove("OUT_DIR"); // rust's default decision for OUT_DIR is perfectly acceptable to PL/Rust
    command.env_remove("RUSTC_WRAPPER"); // plrustc doesn't like being invoked with RUSTC_WRAPPER set.
    command.env_remove("RUSTC_WORKSPACE_WRAPPER"); // ditto.
}

/// Asks Postgres, via FFI, for all of its compile-time configuration data.  This is the full
/// set of things that Postgres' `pg_config` configuration tool can report.
///
/// The returned tuple is a `(key, value)` pair of the configuration name and its value.
fn pg_config_values() -> impl Iterator<Item = (String, String)> {
    unsafe {
        // SAFETY:  we know the memory context we're switching to is valid because we're also making
        // it right here.  We're also responsible for pfreeing the result of `get_configdata()` and
        // the easiest way to do that is to simply free an entire memory context at once
        PgMemoryContexts::new("configdata").switch_to(|_| {
            let mut nprops = 0;
            // SAFETY:  `get_configdata` needs to know where the "postmaster" executable is located
            // and `pg_sys::my_exec_path` is that global, which Postgres assigns once early in its
            // startup process
            let configdata = pg_sys::get_configdata(pg_sys::my_exec_path.as_ptr(), &mut nprops);

            // SAFETY:  `get_configdata` will never return the NULL pointer
            let slice = std::slice::from_raw_parts(configdata, nprops);

            let mut values = Vec::with_capacity(nprops);
            for e in slice {
                // SAFETY:  the members (we use) in `ConfigData` are properly allocated char pointers,
                // done so by the `get_configdata()` call above
                let name = CStr::from_ptr(e.name);
                let setting = CStr::from_ptr(e.setting);
                values.push((
                    name.to_string_lossy().to_string(),
                    setting.to_string_lossy().to_string(),
                ))
            }

            values.into_iter()
        })
    }
}
