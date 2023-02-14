use pgx::pg_sys;

use crate::error::PlRustError;
use crate::user_crate::lint::{required_lints, LintSet};
use crate::user_crate::{CrateState, FnReady};

pub(crate) struct FnValidate {
    pg_proc_xmin: pg_sys::TransactionId,
    db_oid: pg_sys::Oid,
    fn_oid: pg_sys::Oid,
    shared_object: Vec<u8>,
}

impl CrateState for FnValidate {}

impl FnValidate {
    #[tracing::instrument(level = "debug", skip_all)]
    pub(crate) fn new(
        pg_proc_xmin: pg_sys::TransactionId,
        db_oid: pg_sys::Oid,
        fn_oid: pg_sys::Oid,
        shared_object: Vec<u8>,
        lints: LintSet,
    ) -> eyre::Result<Self> {
        // if the set of lints we're validating don't include every required lint, we raise an error
        // with the missing lints
        let missing_lints = required_lints()
            .difference(&lints)
            .cloned()
            .collect::<LintSet>();
        if missing_lints.len() > 0 {
            return Err(eyre::eyre!(PlRustError::MissingLints(missing_lints)));
        }

        Ok(Self {
            pg_proc_xmin,
            db_oid,
            fn_oid,
            shared_object,
        })
    }

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
