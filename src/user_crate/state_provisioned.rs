use crate::{
    user_crate::{target, CrateState, StateBuilt},
    PlRustError,
};
use color_eyre::{Section, SectionExt};
use eyre::{eyre, WrapErr};
use pgx::pg_sys;
use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
};

#[must_use]
pub(crate) struct StateProvisioned {
    fn_oid: pg_sys::Oid,
    crate_name: String,
    crate_dir: PathBuf,
}

impl CrateState for StateProvisioned {}

impl StateProvisioned {
    #[tracing::instrument(level = "debug", skip_all, fields(fn_oid = %fn_oid, crate_name = %crate_name, crate_dir = %crate_dir.display()))]
    pub(crate) fn new(fn_oid: pg_sys::Oid, crate_name: String, crate_dir: PathBuf) -> Self {
        Self {
            fn_oid,
            crate_name,
            crate_dir,
        }
    }
    #[tracing::instrument(
        level = "debug",
        skip_all,
        fields(
            fn_oid = %self.fn_oid,
            crate_dir = %self.crate_dir.display(),
            target_dir = target_dir.map(|v| tracing::field::display(v.display())),
        ))]
    pub(crate) fn build(
        self,
        artifact_dir: &Path,
        pg_config: PathBuf,
        target_dir: Option<&Path>,
    ) -> eyre::Result<(StateBuilt, Output)> {
        let mut command = Command::new("cargo");
        let target = target::tuple()?;
        let target_str = &target;

        command.current_dir(&self.crate_dir);
        command.arg("rustc");
        command.arg("--release");
        command.arg("--target");
        command.arg(target_str);
        command.env("PGX_PG_CONFIG_PATH", pg_config);
        if let Some(target_dir) = target_dir {
            command.env("CARGO_TARGET_DIR", &target_dir);
        }
        command.env(
            "RUSTFLAGS",
            "-Ctarget-cpu=native -Clink-args=-Wl,-undefined,dynamic_lookup",
        );

        let output = command.output().wrap_err("`cargo` execution failure")?;

        if output.status.success() {
            use std::env::consts::DLL_SUFFIX;

            let crate_name = self.crate_name;

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

            let built_shared_object_name = &format!("lib{crate_name}{DLL_SUFFIX}");
            let built_shared_object = target_dir
                .map(|d| {
                    d.join(target_str)
                        .join("release")
                        .join(&built_shared_object_name)
                })
                .unwrap_or(
                    self.crate_dir
                        .join("target")
                        .join(target_str)
                        .join("release")
                        .join(built_shared_object_name),
                );

            let mut shared_object_name = crate_name.clone();

            shared_object_name.push_str(DLL_SUFFIX);

            let shared_object = artifact_dir.join(&shared_object_name);

            std::fs::rename(&built_shared_object, &shared_object).wrap_err_with(|| {
                eyre!(
                    "renaming shared object from `{}` to `{}`",
                    built_shared_object.display(),
                    shared_object.display()
                )
            })?;

            Ok((StateBuilt::new(self.fn_oid, shared_object), output))
        } else {
            let stdout =
                String::from_utf8(output.stdout).wrap_err("`cargo`'s stdout was not  UTF-8")?;
            let stderr =
                String::from_utf8(output.stderr).wrap_err("`cargo`'s stderr was not  UTF-8")?;

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

    pub(crate) fn fn_oid(&self) -> &u32 {
        &self.fn_oid
    }

    pub(crate) fn crate_dir(&self) -> &Path {
        &self.crate_dir
    }
}
