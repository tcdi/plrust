/*
Portions Copyright 2021-2025 Technology Concepts & Design, Inc.

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/
use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass, LintContext};

declare_plrust_lint!(
    pub(crate) PLRUST_STDIO,
    "Disallow functions like `io::{stdout, stderr, stdin}`",
);

rustc_lint_defs::declare_lint_pass!(PlrustPrintFunctions => [PLRUST_STDIO]);

impl<'tcx> LateLintPass<'tcx> for PlrustPrintFunctions {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &hir::Expr) {
        let paths: &[&[&str]] = &[
            &["std", "io", "stdio", "stdout"],
            &["std", "io", "stdio", "stderr"],
            &["std", "io", "stdio", "stdin"],
        ];
        for &path in paths {
            if super::utils::does_expr_call_path(cx, expr, path) {
                cx.lint(
                    PLRUST_STDIO,
                    "the standard streams are forbidden in PL/Rust, \
                    consider using `pgrx::log!()` instead",
                    |b| b.set_span(expr.span),
                );
            }
        }
    }
}
