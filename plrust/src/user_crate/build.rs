use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
};

use color_eyre::{Section, SectionExt};
use eyre::{eyre, WrapErr};
use pgx::pg_sys;

use crate::gucs::{compilation_targets, CompilationTarget};
use crate::plrust_proc::get_host_compilation_target;
use crate::{
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
    pg_config: PathBuf,
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
        pg_config: PathBuf,
    ) -> Self {
        Self {
            pg_proc_xmin,
            db_oid,
            fn_oid,
            crate_name,
            crate_dir,
            pg_config,
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
        let (this_target, other_targets) = compilation_targets();
        let mut results = Vec::new();

        // always build for this host machine
        results.push(self.build_internal(target_dir, this_target)?);

        // and then do the others, which is guaranteed not to contain the exact same triple as `this_target`
        for other_target in other_targets {
            results.push(self.build_internal(target_dir, other_target)?);
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
            target_dir = tracing::field::display(target_dir.display()),
            target_triple = %target_triple,
        ))]
    fn build_internal(
        &self,
        target_dir: &Path,
        target_triple: CompilationTarget,
    ) -> eyre::Result<(FnLoad, Output)> {
        let mut command = Command::new("cargo");

        command.current_dir(&self.crate_dir);
        command.arg("rustc");
        command.arg("--release");
        command.arg("--target");
        command.arg(&target_triple);
        command.env("PGX_PG_CONFIG_PATH", &self.pg_config);
        command.env("CARGO_TARGET_DIR", &target_dir);
        command.env("RUSTFLAGS", "-Clink-args=-Wl,-undefined,dynamic_lookup");

        // don't specify a linker if the target we're compiling for is the host's target.  This
        // ensures that in non-cross-compilation installs, the host does **NOT** need a cross-compile
        // toolchain
        if &get_host_compilation_target() != &target_triple {
            command.env(
                &format!(
                    "CARGO_TARGET_{}_LINKER",
                    &target_triple.as_str().replace('-', "_").to_uppercase()
                ),
                // the value for this variable most likely ends with `-gcc` and also dosn't have
                // the `-unknown-` bit in the middle
                &format!("{}-gcc", target_triple.replace("-unknown-", "-")),
            );
        }

        let output = command.output().wrap_err("`cargo` execution failure")?;

        if output.status.success() {
            let crate_name = &self.crate_name;

            #[cfg(any(
                all(target_os = "macos", target_arch = "x86_64"),
                feature = "force_enable_x86_64_darwin_generations"
            ))]
            let crate_name = {
                let mut crate_name = crate_name;
                let next = crate::generation::next_generation(&crate_name, true)
                    .map(|gen_num| gen_num)
                    .unwrap_or_default();

                crate_name.push_str(&format!("_{}", next));
                crate_name
            };

            let so_bytes = {
                use std::env::consts::DLL_SUFFIX;
                let so_filename = &format!("lib{crate_name}{DLL_SUFFIX}");
                let so_path = target_dir
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
                ),
                output,
            ))
        } else {
            let stdout = String::from_utf8(output.stdout).wrap_err("cargo stdout was not UTF-8")?;
            let stderr = String::from_utf8(output.stderr).wrap_err("cargo stderr was not UTF-8")?;

            Err(eyre!(PlRustError::CargoBuildFail)
                .section(stdout.header("`cargo build` stdout:"))
                .section(stderr.header("`cargo build` stderr:"))
                .with_section(|| {
                    std::fs::read_to_string(&self.crate_dir.join("src").join("lib.rs"))
                        .wrap_err("Writing generated `lib.rs`")
                        .expect("Reading generated `lib.rs` to output during error")
                        .header("Source Code:")
                }))?
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
