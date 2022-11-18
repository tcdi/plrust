/*
Portions Copyright 2020-2021 ZomboDB, LLC.
Portions Copyright 2021-2022 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

use once_cell::sync::Lazy;
use pgx::guc::{GucContext, GucRegistry, GucSetting};
use std::path::PathBuf;
use std::str::FromStr;

static PLRUST_WORK_DIR: GucSetting<Option<&'static str>> = GucSetting::new(None);
static PLRUST_PG_CONFIG: GucSetting<Option<&'static str>> = GucSetting::new(None);
static PLRUST_TRACING_LEVEL: GucSetting<Option<&'static str>> = GucSetting::new(None);
pub(crate) static PLRUST_ALLOWED_DEPENDENCIES: GucSetting<Option<&'static str>> =
    GucSetting::new(None);

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
        GucContext::Sighup,
    );
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
