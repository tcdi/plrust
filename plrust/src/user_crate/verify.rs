/*
Copyright 2021-2023 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

/*!
Provisioned and ready for validation steps

To detect unsafe code in PL/Rust while still using PGX requires some circumlocution.
PGX creates `#[no_mangle] unsafe extern "C" fn` wrappers that allow Postgres to call Rust,
as PostgreSQL will dynamically load what it thinks is a C library and call C ABI wrapper fn
that themselves handle the Postgres fn call ABI for the programmer and then, finally,
call into the programmer's Rust ABI fn!
This blocks simply using rustc's `unsafe` detection as pgx-macros generated code is unsafe.

The circumlocution is brutal, simple, and effective:
pgx-macros wraps actual Rust which can be safe if it contains no unsafe code!
Such code is powerless (it really, truly, will not run, and may not even build)
but it should still typecheck. Writing an empty shell function first
allows using the linting power of rustc on it as a validation step.
Then the function can be rewritten with annotations from pgx-macros injected.
*/

use crate::user_crate::cargo::cargo;
use crate::user_crate::{CrateState, FnBuild, PlRustError};
use eyre::{eyre, WrapErr};
use pgx::pg_sys;
use std::{
    path::{Path, PathBuf},
    process::Output,
};

/// Available and ready-to-validate PL/Rust crate
///
/// - Requires: a provisioned Cargo crate directory
/// - Produces: verified Rust source code
#[must_use]
pub(crate) struct FnVerify {
    pg_proc_xmin: pg_sys::TransactionId,
    db_oid: pg_sys::Oid,
    fn_oid: pg_sys::Oid,
    crate_name: String,
    crate_dir: PathBuf,
}

impl CrateState for FnVerify {}

impl FnVerify {
    #[tracing::instrument(level = "debug", skip_all, fields(db_oid = %db_oid, fn_oid = %fn_oid, crate_name = %crate_name, crate_dir = %crate_dir.display()))]
    pub(crate) fn new(
        pg_proc_xmin: pg_sys::TransactionId,
        db_oid: pg_sys::Oid,
        fn_oid: pg_sys::Oid,
        crate_name: String,
        crate_dir: PathBuf,
    ) -> Self {
        Self {
            pg_proc_xmin,
            db_oid,
            fn_oid,
            crate_name,
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
            target_dir = tracing::field::display(cargo_target_dir.display()),
        ))]
    pub(crate) fn validate(self, cargo_target_dir: &Path) -> eyre::Result<(FnBuild, Output)> {
        // This is the step which would be used for running validation
        // after writing the lib.rs but before actually building it.
        // As PL/Rust is not fully configured to run user commands here,
        // this version check just smoke-tests the ability to run a command
        let mut command = cargo(cargo_target_dir, None)?;
        command.arg("--version");
        command.arg("--verbose");

        let output = command.output().wrap_err("verification failure")?;

        if output.status.success() {
            Ok((
                FnBuild::new(
                    self.pg_proc_xmin,
                    self.db_oid,
                    self.fn_oid,
                    self.crate_name,
                    self.crate_dir,
                ),
                output,
            ))
        } else {
            Err(eyre!(PlRustError::CargoBuildFail))
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
