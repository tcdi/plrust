/*
Portions Copyright 2021-2025 Technology Concepts & Design, Inc.

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE file.
*/
use pgrx::pg_sys;

use crate::error::PlRustError;
use crate::user_crate::lint::{required_lints, LintSet};
use crate::user_crate::{CrateState, FnReady};

pub(crate) struct FnValidate {
    generation_number: u64,
    db_oid: pg_sys::Oid,
    fn_oid: pg_sys::Oid,
    symbol: Option<String>,
    shared_object: Vec<u8>,
}

impl CrateState for FnValidate {}

impl FnValidate {
    #[tracing::instrument(level = "debug", skip_all)]
    pub(crate) fn new(
        generation_number: u64,
        db_oid: pg_sys::Oid,
        fn_oid: pg_sys::Oid,
        symbol: Option<String>,
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
            generation_number,
            db_oid,
            fn_oid,
            symbol,
            shared_object,
        })
    }

    pub(crate) unsafe fn load(self) -> eyre::Result<FnReady> {
        unsafe {
            // SAFETY:  Caller is responsible for ensuring self.shared_object points to the proper
            // shared library to be loaded
            FnReady::load(
                self.generation_number,
                self.db_oid,
                self.fn_oid,
                self.symbol,
                self.shared_object,
            )
        }
    }
}
