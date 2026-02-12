/*
Portions Copyright 2021-2025 Technology Concepts & Design, Inc.

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE file.
*/
use rustc_ast as ast;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_span::Span;

declare_plrust_lint!(
    pub(crate) PLRUST_ASYNC,
    "Disallow use of async and await",
);

rustc_lint_defs::declare_lint_pass!(PlrustAsync => [PLRUST_ASYNC]);

impl EarlyLintPass for PlrustAsync {
    fn check_expr(&mut self, cx: &EarlyContext, expr: &ast::Expr) {
        if let ast::ExprKind::Async(..) | ast::ExprKind::Await(..) = &expr.kind {
            cx.lint(
                PLRUST_ASYNC,
                "Use of async/await is forbidden in PL/Rust",
                |b| b.set_span(expr.span),
            );
        }
    }
    fn check_fn(
        &mut self,
        cx: &EarlyContext,
        kind: ast::visit::FnKind<'_>,
        span: Span,
        _: ast::NodeId,
    ) {
        if let Some(h) = kind.header() {
            if h.asyncness.is_async() {
                cx.lint(
                    PLRUST_ASYNC,
                    "Use of async/await is forbidden in PL/Rust",
                    |b| b.set_span(span),
                );
            }
        }
    }
}
