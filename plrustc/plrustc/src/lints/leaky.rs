/*
Portions Copyright 2021-2025 Technology Concepts & Design, Inc.

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/
use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass, LintContext};

declare_plrust_lint!(
    pub(crate) PLRUST_LEAKY,
    "Disallow use of `{Box,Vec,String}::leak`, `mem::forget`, and similar functions",
);

rustc_lint_defs::declare_lint_pass!(PlrustLeaky => [PLRUST_LEAKY]);

impl<'tcx> LateLintPass<'tcx> for PlrustLeaky {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &hir::Expr) {
        let paths: &[&[&str]] = &[
            &["alloc", "boxed", "Box", "leak"],
            &["alloc", "vec", "Vec", "leak"],
            &["alloc", "string", "String", "leak"],
            &["core", "mem", "forget"],
        ];
        for &path in paths {
            if super::utils::does_expr_call_path(cx, expr, path) {
                cx.lint(
                    PLRUST_LEAKY,
                    "Leaky functions are forbidden in PL/Rust",
                    |b| b.set_span(expr.span),
                );
            }
        }
    }
}
