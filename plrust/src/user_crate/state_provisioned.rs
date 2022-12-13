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

use crate::user_crate::{target, CrateState, CrateVariant, PlRustError, FnBuild};
use color_eyre::{Section, SectionExt};
use eyre::{eyre, WrapErr};
use pgx::pg_sys;
use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
};

/// Provisioned and ready to validate
#[must_use]
pub(crate) struct FnVerify {
    pg_proc_xmin: pg_sys::TransactionId,
    db_oid: pg_sys::Oid,
    fn_oid: pg_sys::Oid,
    crate_name: String,
    crate_dir: PathBuf,
    user_fn: syn::ItemFn,
    variant: CrateVariant,
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
        user_fn: syn::ItemFn,
        variant: CrateVariant,
    ) -> Self {
        Self {
            pg_proc_xmin,
            db_oid,
            fn_oid,
            crate_name,
            crate_dir,
            user_fn,
            variant,
        }
    }

    // TODO: Maybe should be in next state? Prevent access until validated?
    // This only would be very useful if it was also module-abstracted.
    #[tracing::instrument(level = "debug", skip_all, fields(db_oid = %self.db_oid, fn_oid = %self.fn_oid))]
    pub(crate) fn unsafe_lib_rs(&self) -> eyre::Result<syn::File> {
        let mut skeleton: syn::File = syn::parse_quote!(
            #![deny(unsafe_op_in_unsafe_fn)]
            use pgx::prelude::*;
        );

        let crate_name = &self.crate_name;
        tracing::trace!(symbol_name = %crate_name, "Generating `lib.rs` for build step");

        let mut user_fn = self.user_fn.clone();
        match &self.variant {
            CrateVariant::Function { .. } => {
                user_fn.attrs.push(syn::parse_quote! {
                    #[pg_extern]
                });
            }
            CrateVariant::Trigger => {
                user_fn.attrs.push(syn::parse_quote! {
                    #[pg_trigger]
                });
            }
        };

        skeleton.items.push(user_fn.into());
        Ok(skeleton)
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
    pub(crate) fn validate(
        self,
        pg_config: PathBuf,
        target_dir: &Path,
    ) -> eyre::Result<(FnBuild, Output)> {
        let mut command = Command::new("cargo");
        let target = target::tuple()?;
        let target_str = &target;

        command.current_dir(&self.crate_dir);
        command.arg("check");
        command.arg("--target");
        command.arg(target_str);
        command.env("PGX_PG_CONFIG_PATH", &pg_config);
        command.env("CARGO_TARGET_DIR", &target_dir);
        command.env("RUSTFLAGS", "-Clink-args=-Wl,-undefined,dynamic_lookup");

        let output = command.output().wrap_err("`cargo` execution failure")?;

        // TODO: Maybe this should instead be an explicit re-provisioning step?
        if output.status.success() {
            let crate_name = self.crate_name.clone();

            // rebuild code:
            let lib_rs = self.unsafe_lib_rs()?;
            let lib_rs_path = self.crate_dir.join("src/lib.rs");
            std::fs::write(&lib_rs_path, &prettyplease::unparse(&lib_rs))
                .wrap_err("Writing generated `lib.rs`")?;

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

            Ok((
                FnBuild::new(
                    self.pg_proc_xmin,
                    self.db_oid,
                    self.fn_oid,
                    crate_name,
                    self.crate_dir,
                    pg_config,
                ),
                output,
            ))
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

    pub(crate) fn fn_oid(&self) -> pg_sys::Oid {
        self.fn_oid
    }

    pub(crate) fn db_oid(&self) -> pg_sys::Oid {
        self.db_oid
    }
    pub(crate) fn crate_dir(&self) -> &Path {
        &self.crate_dir
    }
}
