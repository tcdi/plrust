/*
Copyright 2021-2023 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

use std::ffi::CStr;
use std::{
    path::{Path, PathBuf},
    process::Output,
};

use color_eyre::{Section, SectionExt};
use eyre::{eyre, WrapErr};
use pgx::{pg_sys, PgMemoryContexts};

use crate::target::{CompilationTarget, CrossCompilationTarget};
use crate::user_crate::cargo;
use crate::{
    gucs,
    user_crate::{CrateState, FnLoad},
    PlRustError,
};

/// Build the dynamic library from source
///
/// - Requires: PL/Rust && Rust source verification
/// - Produces: a dlopenable artifact
#[must_use]
pub(crate) struct FnBuild {
    generation_number: u64,
    db_oid: pg_sys::Oid,
    fn_oid: pg_sys::Oid,
    crate_dir: PathBuf,
}

impl CrateState for FnBuild {}

impl FnBuild {
    #[tracing::instrument(level = "debug", skip_all, fields(db_oid = %db_oid, fn_oid = %fn_oid, crate_name = %crate_name, crate_dir = %crate_dir.display()))]
    pub(crate) fn new(
        generation_number: u64,
        db_oid: pg_sys::Oid,
        fn_oid: pg_sys::Oid,
        crate_name: String,
        crate_dir: PathBuf,
    ) -> Self {
        Self {
            generation_number,
            db_oid,
            fn_oid,
            crate_dir,
        }
    }

    #[tracing::instrument(
        level = "debug",
        skip_all,
        fields(
            db_oid = %self.db_oid,
            fn_oid = %self.fn_oid,
            crate_dir = %self.crate_dir.display(),
            target_dir = tracing::field::display(target_dir.display()),
        ))]
    pub(crate) fn build(self, target_dir: &Path) -> eyre::Result<Vec<(FnLoad, Output)>> {
        let (this_target, cross_compilation_targets) = gucs::compilation_targets()?;
        let mut results = Vec::new();

        // always build for this host machine
        results.push(self.build_internal(target_dir, this_target.clone(), None)?);

        // and then do the others, which is guaranteed not to contain the exact same triple as `this_target`
        for target in cross_compilation_targets {
            results.push(self.build_internal(target_dir, target.target(), Some(target))?);
        }
        Ok(results)
    }

    #[tracing::instrument(
        level = "debug",
        skip_all,
        fields(
            db_oid = %self.db_oid,
            fn_oid = %self.fn_oid,
            crate_dir = %self.crate_dir.display(),
            target_dir = tracing::field::display(cargo_target_dir.display()),
            target_triple = %target_triple,
            cross_compilation_target
        ))]
    fn build_internal(
        &self,
        cargo_target_dir: &Path,
        target_triple: CompilationTarget,
        cross_compilation_target: Option<CrossCompilationTarget>,
    ) -> eyre::Result<(FnLoad, Output)> {
        let mut command = cargo()?;

        command.current_dir(&self.crate_dir);
        command.arg("rustc");
        command.arg("--release");
        command.arg("--target");
        command.arg(&target_triple);
        command.env("CARGO_TARGET_DIR", &cargo_target_dir);
        command.env("RUSTFLAGS", "-Clink-args=-Wl,-undefined,dynamic_lookup");

        // There was a time in the past where plrust had a `plrust.pg_config` GUC whose value was
        // passed down to the "pgx-pg-sys" transient dependency via an environment variable.
        //
        // This turned out to be an unwanted bit of user, system, and operational complexity.
        //
        // Instead, we tell the environment that "pg_config" is described as environment
        // variables, and set every property Postgres can tell us (which is essentially how
        // `pg_config` itself works) as individual environment variables, each prefixed with
        // "PGX_PG_CONFIG_".
        //
        // "pgx-pg-sys"'s build.rs knows how to interpret these environment variables to get what
        // it needs to properly generate bindings.
        command.env("PGX_PG_CONFIG_AS_ENV", "true");
        for (k, v) in pg_config_values() {
            let k = format!("PGX_PG_CONFIG_{k}");
            command.env(k, v);
        }

        // set environment variables we need in order for a cross compile
        if let Some(target_triple) = cross_compilation_target {
            // the CARGO_TARGET_xx_LINKER variable
            let (k, v) = target_triple.linker_envar();
            command.env(k, v);

            // pgx-specified variable for where the bindings are
            if let Some((k, v)) = target_triple.bindings_envar() {
                command.env(k, v);
            }
        }

        let output = command.output().wrap_err("`cargo` execution failure")?;

        if output.status.success() {
            let so_bytes = {
                let crate_name =
                    crate::plrust::crate_name(self.db_oid, self.fn_oid, self.generation_number);
                use std::env::consts::DLL_SUFFIX;
                let so_filename = &format!("lib{crate_name}{DLL_SUFFIX}");
                let so_path = cargo_target_dir
                    .join(&target_triple)
                    .join("release")
                    .join(&so_filename);

                std::fs::read(&so_path)?
            };

            Ok((
                FnLoad::new(
                    self.generation_number,
                    self.db_oid,
                    self.fn_oid,
                    target_triple,
                    Some(crate::plrust::symbol_name(self.db_oid, self.fn_oid)),
                    so_bytes,
                ),
                output,
            ))
        } else {
            let stdout = String::from_utf8(output.stdout).wrap_err("cargo stdout was not UTF-8")?;
            let stderr = String::from_utf8(output.stderr).wrap_err("cargo stderr was not UTF-8")?;

            let err = Err(eyre!(PlRustError::CargoBuildFail)
                .section(stdout.header("`cargo build` stdout:"))
                .section(stderr.header("`cargo build` stderr:"))
                .with_section(|| {
                    std::fs::read_to_string(&self.crate_dir.join("src").join("lib.rs"))
                        .wrap_err("Writing generated `lib.rs`")
                        .expect("Reading generated `lib.rs` to output during error")
                        .header("Source Code:")
                }));

            // Clean up on error but don't let this error replace our user's error!
            if let Err(e) = std::fs::remove_dir_all(&self.crate_dir) {
                pgx::log!("Problem during removing crate directory: {e}")
            };

            err?
        }
    }

    // for #[tracing] purposes
    pub(crate) fn fn_oid(&self) -> pg_sys::Oid {
        self.fn_oid
    }

    // for #[tracing] purposes
    pub(crate) fn db_oid(&self) -> pg_sys::Oid {
        self.db_oid
    }

    // for #[tracing] purposes
    pub(crate) fn crate_dir(&self) -> &Path {
        &self.crate_dir
    }
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
