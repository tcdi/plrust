/*
Copyright 2021-2023 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

/*!
Provisioned and ready for validation steps

To detect unsafe code in PL/Rust while still using PGRX requires some circumlocution.
PGRX creates `#[no_mangle] unsafe extern "C" fn` wrappers that allow Postgres to call Rust,
as PostgreSQL will dynamically load what it thinks is a C library and call C ABI wrapper fn
that themselves handle the Postgres fn call ABI for the programmer and then, finally,
call into the programmer's Rust ABI fn!
This blocks simply using rustc's `unsafe` detection as pgrx-macros generated code is unsafe.

The circumlocution is brutal, simple, and effective:
pgrx-macros wraps actual Rust which can be safe if it contains no unsafe code!
Such code is powerless (it really, truly, will not run, and may not even build)
but it should still typecheck. Writing an empty shell function first
allows using the linting power of rustc on it as a validation step.
Then the function can be rewritten with annotations from pgrx-macros injected.
*/

use std::{
    path::{Path, PathBuf},
    process::Output,
};

use color_eyre::{Section, SectionExt};
use eyre::{eyre, WrapErr};
use pgrx::pg_sys;

use crate::gucs;
use crate::target::{CompilationTarget, CrossCompilationTarget};
use crate::user_crate::build::set_plrustc_vars;
use crate::user_crate::cargo::cargo;
use crate::user_crate::lint::LintSet;
use crate::user_crate::{CrateState, FnBuild, PlRustError};

/// Available and ready-to-validate PL/Rust crate
///
/// - Requires: a provisioned Cargo crate directory
/// - Produces: verified Rust source code
#[must_use]
pub(crate) struct FnVerify {
    generation_number: u64,
    db_oid: pg_sys::Oid,
    fn_oid: pg_sys::Oid,
    crate_name: String,
    crate_dir: PathBuf,
    lints: LintSet,
}

impl CrateState for FnVerify {}

impl FnVerify {
    #[tracing::instrument(level = "debug", skip_all, fields(db_oid = %db_oid, fn_oid = %fn_oid, crate_name = %crate_name, crate_dir = %crate_dir.display()))]
    pub(crate) fn new(
        generation_number: u64,
        db_oid: pg_sys::Oid,
        fn_oid: pg_sys::Oid,
        crate_name: String,
        crate_dir: PathBuf,
        lints: LintSet,
    ) -> Self {
        Self {
            generation_number,
            db_oid,
            fn_oid,
            crate_name,
            crate_dir,
            lints,
        }
    }

    fn user_crate_name(&self) -> String {
        crate::plrust::crate_name(self.db_oid, self.fn_oid, self.generation_number)
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
    pub(crate) fn validate(self, cargo_target_dir: &Path) -> eyre::Result<(FnBuild, Vec<Output>)> {
        // This is the step which would be used for running validation
        // after writing the lib.rs but before actually building it.
        // As PL/Rust is not fully configured to run user commands here,
        // this version check just smoke-tests the ability to run a command
        let (this_target, cross_compilation_targets) = gucs::compilation_targets()?;
        let mut output = Vec::new();
        output.push(self.check_internal(cargo_target_dir, this_target.clone(), None));

        for target in cross_compilation_targets {
            output.push(self.check_internal(cargo_target_dir, target.target(), Some(target)));
        }

        let output = output.into_iter().collect::<eyre::Result<Vec<Output>>>();
        output.map(|v| {
            (
                FnBuild::new(
                    self.generation_number,
                    self.db_oid,
                    self.fn_oid,
                    self.crate_name,
                    self.crate_dir,
                    self.lints,
                ),
                v,
            )
        })
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
    fn check_internal(
        &self,
        cargo_target_dir: &Path,
        target_triple: CompilationTarget,
        cross_compilation_target: Option<CrossCompilationTarget>,
    ) -> eyre::Result<Output> {
        let mut command = cargo(cargo_target_dir, cross_compilation_target)?;
        let user_crate_name = self.user_crate_name();
        set_plrustc_vars(
            &mut command,
            &user_crate_name,
            &self.crate_dir,
            &cargo_target_dir,
        )?;

        command.current_dir(&self.crate_dir);
        command.arg("check");
        command.arg("--target");
        command.arg(&target_triple);
        command.arg("--features");
        command.arg("check_forbidden");

        let output = command.output().wrap_err("`cargo` execution failure")?;

        if output.status.success() {
            Ok(output)
        } else {
            let stdout = String::from_utf8(output.stdout).wrap_err("cargo stdout was not UTF-8")?;
            let stderr = String::from_utf8(output.stderr).wrap_err("cargo stderr was not UTF-8")?;

            let err = Err(eyre!(PlRustError::CargoBuildFail)
                .section(stdout.header("`cargo check` stdout:"))
                .section(stderr.header("`cargo check` stderr:"))
                .with_section(|| {
                    std::fs::read_to_string(&self.crate_dir.join("src").join("lib.rs"))
                        .wrap_err("Writing generated `lib.rs`")
                        .expect("Reading generated `lib.rs` to output during error")
                        .header("Source Code:")
                }));

            // Clean up on error but don't let this error replace our user's error!
            if let Err(e) = std::fs::remove_dir_all(&self.crate_dir) {
                pgrx::log!("Problem during removing crate directory: {e}")
            };

            err?
        }
    }
}
