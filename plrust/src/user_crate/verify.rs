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

use crate::user_crate::{target, CrateState, FnBuild, PlRustError};
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
            target_dir = tracing::field::display(target_dir.display()),
        ))]
    pub(crate) fn validate(
        self,
        pg_config: PathBuf,
        target_dir: &Path,
    ) -> eyre::Result<(FnBuild, Output)> {
        // This is the step which would be used for running validation
        // after writing the lib.rs but before actually building it.
        // As PL/Rust is not fully configured to run user commands here,
        // this just echoes to smoke-test the ability to run a command
        let mut command = Command::new("echo");
        let target = target::tuple()?;
        let target_str = &target;

        let args = format!(
            r#"'
            --target {target_str}
            PGX_PG_CONFIG_PATH = {config}
            CARGO_TARGET_DIR = {dir}
            RUSTFLAGS = -Clink-args=-Wl,-undefined,dynamic_lookup'"#,
            config = pg_config.display(),
            dir = target_dir.display()
        );

        command.current_dir(&self.crate_dir);
        command.arg(args);

        let output = command.output().wrap_err("verification failure")?;

        if output.status.success() {
            Ok((
                FnBuild::new(
                    self.pg_proc_xmin,
                    self.db_oid,
                    self.fn_oid,
                    self.crate_name,
                    self.crate_dir,
                    pg_config,
                ),
                output,
            ))
        } else {
            Err(eyre!(PlRustError::CargoBuildFail))
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