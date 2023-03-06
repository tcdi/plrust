/*
Copyright 2021-2023 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
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
    generation_number: u64,
    db_oid: pg_sys::Oid,
    fn_oid: pg_sys::Oid,
    crate_dir: PathBuf,
    lints: LintSet,
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
        lints: LintSet,
    ) -> Self {
        Self {
            generation_number,
            db_oid,
            fn_oid,
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
        set_plrustc_vars(&mut command, self, cargo_target_dir)?;

        command.current_dir(&self.crate_dir);
        command.arg("rustc");
        command.arg("--release");
        command.arg("--target");
        command.arg(&target_triple);

        let output = command.output().wrap_err("`cargo` execution failure")?;

        if output.status.success() {
            let so_bytes = {
                let crate_name = self.user_crate_name();
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

// Canonicalize path and ensure UTF-8
fn path2string(p: &Path) -> eyre::Result<String> {
    let pbuf = p.canonicalize().or_else(|_| {
        use omnipath::posix::PosixPathExt;
        p.posix_absolute()
    })?;
    let Some(pathstr) = pbuf.to_str() else {
        eyre::bail!("non-UTF-8 paths are not supported. Got: {pbuf:?}");
    };
    Ok(pathstr.to_owned())
}

fn set_plrustc_vars(command: &mut Command, build: &FnBuild, target_dir: &Path) -> eyre::Result<()> {
    command.env("PLRUSTC_USER_CRATE_NAME", build.user_crate_name());
    let crate_dir_str = path2string(&build.crate_dir)?;
    let target_dir_str = path2string(target_dir)?;

    // TODO: Allow extra dirs via a GUC? Support excluding dirs?
    let allowed_dirs = std::env::join_paths([crate_dir_str, target_dir_str])?;
    command.env("PLRUSTC_USER_CRATE_MAY_ACCESS", allowed_dirs);

    Ok(())
}
