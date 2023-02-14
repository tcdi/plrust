/*
Copyright 2021-2023 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

use pgx::pg_sys;

use crate::target::CompilationTarget;
use crate::user_crate::lint::LintSet;
use crate::user_crate::{CrateState, FnValidate};

/// Available and ready-to-load PL/Rust function
///
/// - Requires: a dlopenable artifact
/// - Produces: a dlopened artifact
#[must_use]
pub(crate) struct FnLoad {
    pg_proc_xmin: pg_sys::TransactionId,
    db_oid: pg_sys::Oid,
    fn_oid: pg_sys::Oid,
    target: CompilationTarget,
    shared_object: Vec<u8>,
    lints: LintSet,
}

impl CrateState for FnLoad {}

impl FnLoad {
    #[tracing::instrument(level = "debug", skip_all)]
    pub(crate) fn new(
        pg_proc_xmin: pg_sys::TransactionId,
        db_oid: pg_sys::Oid,
        fn_oid: pg_sys::Oid,
        target: CompilationTarget,
        shared_object: Vec<u8>,
        lints: LintSet,
    ) -> Self {
        Self {
            pg_proc_xmin,
            db_oid,
            fn_oid,
            target,
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
            self.pg_proc_xmin,
            self.db_oid,
            self.fn_oid,
            self.shared_object,
            self.lints,
        )
    }
}
