use crate::{
    user_crate::{CrateState, StateBuilt},
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
    #[tracing::instrument(level = "debug", skip_all)]
    pub(crate) fn new(fn_oid: pg_sys::Oid, crate_name: String, crate_dir: PathBuf) -> Self {
        Self {
            fn_oid,
            crate_name,
            crate_dir,
        }
    }
    #[tracing::instrument(level = "debug", skip_all)]
    pub(crate) fn build(
        self,
        artifact_dir: &Path,
        pg_config: PathBuf,
        target_dir: Option<&Path>,
    ) -> eyre::Result<(StateBuilt, Output)> {
        let mut command = Command::new("cargo");

        command.current_dir(&self.crate_dir);
        command.arg("rustc");
        command.arg("--release");
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
            let crate_name = &self.crate_name;
            use std::env::consts::DLL_SUFFIX;

            let built_shared_object_name = &format!("lib{crate_name}{DLL_SUFFIX}");
            let built_shared_object = target_dir
                .map(|d| d.join("release").join(&built_shared_object_name))
                .unwrap_or_else(|| {
                    self.crate_dir
                        .join("target")
                        .join("release")
                        .join(built_shared_object_name)
                });

            let mut shared_object_name = crate_name.clone();
            #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
            {
                let latest = crate::generation::latest_generation(&crate_name, true)
                    .map(|(gen_num, _)| gen_num)
                    .unwrap_or_default();

                shared_object_name.push_str(&format!("_{}", latest));
            };
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
}
