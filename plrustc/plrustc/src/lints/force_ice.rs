/*
Portions Copyright 2021-2025 Technology Concepts & Design, Inc.

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE file.
*/
use rustc_ast as ast;
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_span::Span;

// Used to force an ICE in our uitests. Only enabled if
// `PLRUSTC_INCLUDE_TEST_ONLY_LINTS` is enabled in the environment, which we do
// explicitly in the tests that need it.
declare_plrust_lint! {
    pub(crate) PLRUST_TEST_ONLY_FORCE_ICE,
    "This message should not appear in the output"
}

rustc_lint_defs::declare_lint_pass!(PlrustcForceIce => [PLRUST_TEST_ONLY_FORCE_ICE]);

impl EarlyLintPass for PlrustcForceIce {
    fn check_fn(
        &mut self,
        _: &EarlyContext<'_>,
        fn_kind: ast::visit::FnKind<'_>,
        _: Span,
        _: ast::NodeId,
    ) {
        use ast::visit::FnKind;
        const GIMME_ICE: &str = "plrustc_would_like_some_ice";
        if matches!(&fn_kind, FnKind::Fn(_, id, ..) if id.name.as_str() == GIMME_ICE) {
            panic!("Here is your ICE");
        }
    }
}
