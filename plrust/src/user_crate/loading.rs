/*
Copyright 2021-2023 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

use pgrx::pg_sys;

use crate::target::CompilationTarget;
use crate::user_crate::lint::LintSet;
use crate::user_crate::{CrateState, FnValidate};

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
    lints: LintSet,
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
        lints: LintSet,
    ) -> Self {
        Self {
            generation_number,
            db_oid,
            fn_oid,
            target,
            symbol,
            shared_object,
            lints,
        }
    }

    pub(crate) fn into_inner(self) -> (CompilationTarget, Vec<u8>, LintSet) {
        (self.target, self.shared_object, self.lints)
    }

    #[tracing::instrument(level = "debug", skip_all, fields(db_oid = % self.db_oid, fn_oid = % self.fn_oid))]
    pub(crate) unsafe fn validate(self) -> eyre::Result<FnValidate> {
        FnValidate::new(
            self.generation_number,
            self.db_oid,
            self.fn_oid,
            self.symbol,
            self.shared_object,
            self.lints,
        )
    }
}
