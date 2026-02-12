/*
Portions Copyright 2021-2025 Technology Concepts & Design, Inc.

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/
use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::Span;

declare_plrust_lint!(
    pub(crate) PLRUST_PRINT_MACROS,
    "Disallow `print!`, `println!`, `eprint!` and `eprintln!`",
);

rustc_lint_defs::declare_lint_pass!(PlrustPrintMacros => [PLRUST_PRINT_MACROS]);

impl PlrustPrintMacros {
    fn check_span(&self, cx: &LateContext<'_>, srcspan: Span) {
        let diagnostic_items = [
            sym!(print_macro),
            sym!(eprint_macro),
            sym!(println_macro),
            sym!(eprintln_macro),
            sym!(dbg_macro),
        ];
        if let Some((span, _which, _did)) =
            super::utils::check_span_against_macro_diags(cx, srcspan, &diagnostic_items)
        {
            self.fire(cx, span);
        };
    }
    fn fire(&self, cx: &LateContext<'_>, span: Span) {
        cx.lint(
            PLRUST_PRINT_MACROS,
            "the printing macros are forbidden in PL/Rust, \
            consider using `pgrx::log!()` instead",
            |b| b.set_span(span),
        );
    }
}
impl<'tcx> LateLintPass<'tcx> for PlrustPrintMacros {
    fn check_item(&mut self, cx: &LateContext<'tcx>, h: &hir::Item) {
        self.check_span(cx, h.span);
    }
    fn check_stmt(&mut self, cx: &LateContext<'tcx>, h: &hir::Stmt) {
        self.check_span(cx, h.span);
    }
    fn check_expr(&mut self, cx: &LateContext<'tcx>, h: &hir::Expr) {
        self.check_span(cx, h.span);
    }
}
