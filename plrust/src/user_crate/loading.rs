use crate::user_crate::{CrateState, FnReady};
use pgx::pg_sys;
use std::path::{Path, PathBuf};

/// Available and ready-to-load PL/Rust function
///
/// - Requires: a dlopenable artifact
/// - Produces: a dlopened artifact
#[must_use]
pub(crate) struct FnLoad {
    pg_proc_xmin: pg_sys::TransactionId,
    db_oid: pg_sys::Oid,
    fn_oid: pg_sys::Oid,
    shared_object: PathBuf,
}

impl CrateState for FnLoad {}

impl FnLoad {
    #[tracing::instrument(level = "debug", skip_all)]
    pub(crate) fn new(
        pg_proc_xmin: pg_sys::TransactionId,
        db_oid: pg_sys::Oid,
        fn_oid: pg_sys::Oid,
        shared_object: PathBuf,
    ) -> Self {
        Self {
            pg_proc_xmin,
            db_oid,
            fn_oid,
            shared_object,
        }
    }

    pub(crate) fn shared_object(&self) -> &Path {
        &self.shared_object
    }

    pub(crate) fn fn_oid(&self) -> pg_sys::Oid {
        self.fn_oid
    }

    pub(crate) fn db_oid(&self) -> pg_sys::Oid {
        self.db_oid
    }

    #[tracing::instrument(level = "debug", skip_all, fields(db_oid = %self.db_oid, fn_oid = %self.fn_oid, shared_object = %self.shared_object.display()))]
    pub(crate) unsafe fn load(self) -> eyre::Result<FnReady> {
        unsafe {
            // SAFETY:  Caller is responsible for ensuring self.shared_object points to the proper
            // shared library to be loaded
            FnReady::load(
                self.pg_proc_xmin,
                self.db_oid,
                self.fn_oid,
                self.shared_object,
            )
        }
    }
}
