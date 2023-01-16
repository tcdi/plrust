/*
Portions Copyright 2020-2021 ZomboDB, LLC.
Portions Copyright 2021-2022 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

use crate::target;
use crate::target::CompilationTarget;
use once_cell::sync::Lazy;
use pgx::guc::{GucContext, GucRegistry, GucSetting};
use pgx::pg_sys;
use pgx::pg_sys::AsPgCStr;
use std::ffi::CStr;
use std::path::PathBuf;
use std::str::FromStr;

static PLRUST_WORK_DIR: GucSetting<Option<&'static str>> = GucSetting::new(None);
static PLRUST_PG_CONFIG: GucSetting<Option<&'static str>> = GucSetting::new(None);
static PLRUST_TRACING_LEVEL: GucSetting<Option<&'static str>> = GucSetting::new(None);
pub(crate) static PLRUST_ALLOWED_DEPENDENCIES: GucSetting<Option<&'static str>> =
    GucSetting::new(None);
static PLRUST_COMPILATION_TARGETS: GucSetting<Option<&'static str>> = GucSetting::new(None);

pub(crate) static PLRUST_ALLOWED_DEPENDENCIES_CONTENTS: Lazy<toml::value::Table> =
    Lazy::new(|| {
        let path = PathBuf::from_str(
            &PLRUST_ALLOWED_DEPENDENCIES
                .get()
                .expect("plrust.allowed_dependencies is not set in postgresql.conf"),
        )
        .expect("plrust.allowed_dependencies is not a valid path");

        let contents =
            std::fs::read_to_string(&path).expect("Unable to read allow listed dependencies");

        toml::from_str(&contents).expect("Unable to format allow listed dependencies")
    });

pub(crate) fn init() {
    GucRegistry::define_string_guc(
        "plrust.work_dir",
        "The directory where pl/rust will build functions with cargo",
        "The directory where pl/rust will build functions with cargo",
        &PLRUST_WORK_DIR,
        GucContext::Sighup,
    );

    GucRegistry::define_string_guc(
        "plrust.pg_config",
        "What is the full path to the `pg_config` tool for this Postgres installation?",
        "What is the full path to the `pg_config` tool for this Postgres installation?",
        &PLRUST_PG_CONFIG,
        GucContext::Sighup,
    );

    GucRegistry::define_string_guc(
        "plrust.tracing_level",
        "The tracing level to use while running pl/rust",
        "The tracing level to use while running pl/rust. Should be `error`, `warn`, `info`, `debug`, or `trace`",
        &PLRUST_TRACING_LEVEL,
        GucContext::Sighup,
    );

    GucRegistry::define_string_guc(
        "plrust.allowed_dependencies",
        "The full path of a toml file containing crates and versions allowed when creating PL/Rust functions.",
        "The full path of a toml file containing crates and versions allowed when creating PL/Rust functions.",
        &PLRUST_ALLOWED_DEPENDENCIES,
        GucContext::Postmaster,
    );

    GucRegistry::define_string_guc(
        "plrust.compilation_targets",
        "A comma-separated list of rust compilation 'target triples' to compile for",
        "Useful for when it's known a system will replicate to a Postgres server on a different CPU architecutre",
        &PLRUST_COMPILATION_TARGETS,
        GucContext::Postmaster
    )
}

pub(crate) fn work_dir() -> PathBuf {
    PathBuf::from_str(
        &PLRUST_WORK_DIR
            .get()
            .expect("plrust.work_dir is not set in postgresql.conf"),
    )
    .expect("plrust.work_dir is not a valid path")
}

pub(crate) fn pg_config() -> PathBuf {
    PathBuf::from_str(
        &PLRUST_PG_CONFIG
            .get()
            .expect("plrust.pg_config is not set in postgresql.conf"),
    )
    .expect("plrust.pg_config is not a valid path")
}

pub(crate) fn tracing_level() -> tracing::Level {
    PLRUST_TRACING_LEVEL
        .get()
        .map(|v| v.parse().expect("plrust.tracing_level was invalid"))
        .unwrap_or(tracing::Level::INFO)
}

/// Returns the compilation targets a function should be compiled for.
///
/// The return format is `( <This Host's Target Triple>, <Other Configured Target Triples> )`
pub(crate) fn compilation_targets() -> eyre::Result<(
    &'static CompilationTarget,
    impl Iterator<Item = CompilationTarget>,
)> {
    let this_target = target::tuple()?;
    let other_targets = match PLRUST_COMPILATION_TARGETS.get() {
        None => vec![],
        Some(targets) => targets
            .split(',')
            .map(str::trim)
            .filter(|s| s != &this_target.as_str()) // make sure we don't include "this target" in the list of other targets
            .map(|s| s.into())
            .collect::<Vec<_>>(),
    };

    Ok((this_target, other_targets.into_iter()))
}

pub(crate) fn get_linker_for_target(target: &CompilationTarget) -> eyre::Result<String> {
    unsafe {
        let guc_name = format!("plrust.{}_linker", target.as_str().replace('-', "_"));
        let value = pg_sys::GetConfigOption(guc_name.as_pg_cstr(), true, true);
        if value.is_null() {
            Ok(match target.as_str() {
                "aarch64-postgres-linux-gnu" | "aarch64-unknown-linux-gnu" => {
                    "aarch64-linux-gnu-gcc".into()
                }
                "x86_64-postgres-linux-gnu" | "x86_64-unknown-linux-gnu" => {
                    "x86_64-linux-gnu-gcc".into()
                }
                _ => return Err(eyre::eyre!("unrecognized compilation target: {target}")),
            })
        } else {
            let value_cstr = CStr::from_ptr(value);
            Ok(value_cstr.to_string_lossy().to_string())
        }
    }
}
