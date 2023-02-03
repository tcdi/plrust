/*
Copyright 2021-2023 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

use pgx::pg_sys;

use crate::target::CompilationTarget;
use crate::user_crate::{CrateState, FnReady};

/// Available and ready-to-load PL/Rust function
///
/// - Requires: a dlopenable artifact
/// - Produces: a dlopened artifact
#[must_use]
pub(crate) struct FnLoad {
    generation_number: u64,
    db_oid: pg_sys::Oid,
    fn_oid: pg_sys::Oid,
    target: CompilationTarget,
    symbol: Option<String>,
    shared_object: Vec<u8>,
}

impl CrateState for FnLoad {}

impl FnLoad {
    #[tracing::instrument(level = "debug", skip_all)]
    pub(crate) fn new(
        generation_number: u64,
        db_oid: pg_sys::Oid,
        fn_oid: pg_sys::Oid,
        target: CompilationTarget,
        symbol: Option<String>,
        shared_object: Vec<u8>,
    ) -> Self {
        Self {
            generation_number,
            db_oid,
            fn_oid,
            target,
            symbol,
            shared_object,
        }
    }

    pub(crate) fn into_inner(self) -> (CompilationTarget, Vec<u8>) {
        (self.target, self.shared_object)
    }

    #[tracing::instrument(level = "debug", skip_all, fields(db_oid = %self.db_oid, fn_oid = %self.fn_oid))]
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
