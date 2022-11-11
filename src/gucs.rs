/*
Portions Copyright 2020-2021 ZomboDB, LLC.
Portions Copyright 2021-2022 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

use pgx::guc::{GucContext, GucRegistry, GucSetting};
use std::path::PathBuf;
use std::str::FromStr;

static PLRUST_WORK_DIR: GucSetting<Option<&'static str>> = GucSetting::new(None);
static PLRUST_PG_CONFIG: GucSetting<Option<&'static str>> = GucSetting::new(None);
static PLRUST_TRACING_LEVEL: GucSetting<Option<&'static str>> = GucSetting::new(None);
static PLRUST_AUTO_RECOMPILE: GucSetting<bool> = GucSetting::new(false);

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

    GucRegistry::define_bool_guc(
        "plrust.auto_recompile",
        "Recompile function if unable to find shared object file in plrust.work_dir",
        "Recompile function if unable to find shared object file in plrust.work_dir",
        &PLRUST_AUTO_RECOMPILE,
        GucContext::Sighup,
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

pub(crate) fn auto_recompile() -> bool {
    PLRUST_AUTO_RECOMPILE.get()
}
