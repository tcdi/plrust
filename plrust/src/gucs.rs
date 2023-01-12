/*
Portions Copyright 2020-2021 ZomboDB, LLC.
Portions Copyright 2021-2022 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

use crate::plrust_proc::get_host_compilation_target;
use once_cell::sync::Lazy;
use pgx::guc::{GucContext, GucRegistry, GucSetting};
use std::ffi::OsStr;
use std::fmt::{Display, Formatter};
use std::ops::Deref;
use std::path::{Path, PathBuf};
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

#[derive(Debug, Clone, PartialOrd, PartialEq, Hash, Ord, Eq)]
#[repr(transparent)]
pub(crate) struct CompilationTarget(String);
impl Deref for CompilationTarget {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<&str> for CompilationTarget {
    fn from(s: &str) -> Self {
        CompilationTarget(s.into())
    }
}
impl Display for CompilationTarget {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl AsRef<Path> for CompilationTarget {
    fn as_ref(&self) -> &Path {
        Path::new(&self.0)
    }
}
impl AsRef<OsStr> for CompilationTarget {
    fn as_ref(&self) -> &OsStr {
        OsStr::new(&self.0)
    }
}
impl CompilationTarget {
    pub fn as_str(&self) -> &str {
        &self
    }
}

/// Returns the compilation targets a function should be compiled for.
///
/// The return format is `( <This Host's Target Triple>, <Other Configured Target Triples> )`
pub(crate) fn compilation_targets() -> (CompilationTarget, impl Iterator<Item = CompilationTarget>)
{
    let this_target = get_host_compilation_target();
    let other_targets = match PLRUST_COMPILATION_TARGETS.get() {
        None => vec![],
        Some(targets) => targets
            .split(',')
            .map(str::trim)
            .filter(|s| s != &this_target.as_str()) // make sure we don't include "this target" in the list of other targets
            .map(|s| s.into())
            .collect::<Vec<_>>(),
    };

    (this_target.into(), other_targets.into_iter())
}
