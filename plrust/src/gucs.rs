/*
Portions Copyright 2020-2021 ZomboDB, LLC.
Portions Copyright 2021-2022 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

use pgx::*;
use std::path::PathBuf;
use std::str::FromStr;

static PLRUST_WORK_DIR: GucSetting<Option<&'static str>> = GucSetting::new(None);
static PLRUST_CLEANUP: GucSetting<bool> = GucSetting::new(true);

pub(crate) fn init() {
    GucRegistry::define_string_guc(
        "plrust.work_dir",
        "The directory where pl/rust will build functions with cargo",
        "The directory where pl/rust will build functions with cargo",
        &PLRUST_WORK_DIR,
        GucContext::Sighup,
    );
    GucRegistry::define_bool_guc(
        "plrust.cleanup",
        "If pl/rust should cleanup generated create code",
        "If pl/rust should cleanup generated create code. By default it will remove the code after compilation",
        &PLRUST_CLEANUP,
        GucContext::Sighup,
    );
}

pub(crate) fn work_dir() -> PathBuf {
    let work_dir = PathBuf::from_str(
        &PLRUST_WORK_DIR
            .get()
            .expect("plrust.work_dir is not set in postgresql.conf"),
    )
    .expect("plrust.work_dir is not a valid path");

    // create the work dir if it doesn't exist
    if !work_dir.exists() {
        std::fs::create_dir_all(&work_dir)
            .expect("failed to create directory specified by plrust.work_dir");
    }

    work_dir
}

pub(crate) fn cleanup() -> bool {
    PLRUST_CLEANUP.get()
}