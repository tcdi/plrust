/*
Copyright 2021-2023 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

use std::{
    path::{Path, PathBuf},
    process::Output,
};

use color_eyre::{Section, SectionExt};
use eyre::{eyre, WrapErr};
use pgx::pg_sys;

use crate::target::{CompilationTarget, CrossCompilationTarget};
use crate::user_crate::cargo::cargo;
use crate::user_crate::lint::LintSet;
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
    pg_proc_xmin: pg_sys::TransactionId,
    db_oid: pg_sys::Oid,
    fn_oid: pg_sys::Oid,
    crate_name: String,
    crate_dir: PathBuf,
    lints: LintSet,
}

impl CrateState for FnBuild {}

impl FnBuild {
    #[tracing::instrument(level = "debug", skip_all, fields(db_oid = %db_oid, fn_oid = %fn_oid, crate_name = %crate_name, crate_dir = %crate_dir.display()))]
    pub(crate) fn new(
        pg_proc_xmin: pg_sys::TransactionId,
        db_oid: pg_sys::Oid,
        fn_oid: pg_sys::Oid,
        crate_name: String,
        crate_dir: PathBuf,
        lints: LintSet,
    ) -> Self {
        Self {
            pg_proc_xmin,
            db_oid,
            fn_oid,
            crate_name,
            crate_dir,
            lints,
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
        let mut command = cargo(cargo_target_dir, cross_compilation_target)?;

        command.current_dir(&self.crate_dir);
        command.arg("rustc");
        command.arg("--release");
        command.arg("--target");
        command.arg(&target_triple);

        let output = command.output().wrap_err("`cargo` execution failure")?;

        if output.status.success() {
            let crate_name = &self.crate_name;

            #[cfg(any(
                all(target_os = "macos", target_arch = "x86_64"),
                feature = "force_enable_x86_64_darwin_generations"
            ))]
            let crate_name = {
                let mut crate_name = crate_name.clone();
                let next = crate::generation::next_generation(&crate_name, true)
                    .map(|gen_num| gen_num)
                    .unwrap_or_default();

                crate_name.push_str(&format!("_{}", next));
                crate_name
            };

            let so_bytes = {
                let so_filename = &format!("lib{crate_name}{DLL_SUFFIX}");
                let so_path = cargo_target_dir
                    .join(&target_triple)
                    .join("release")
                    .join(&so_filename);

                std::fs::read(&so_path)?
            };

            Ok((
                FnLoad::new(
                    self.pg_proc_xmin,
                    self.db_oid,
                    self.fn_oid,
                    target_triple,
                    so_bytes,
                    self.lints.clone(),
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
