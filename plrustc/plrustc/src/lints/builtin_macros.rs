use super::utils;
use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::{Span, Symbol};

declare_plrust_lint!(
    pub(crate) PLRUST_FILESYSTEM_MACROS,
    "Disallow `include_str!`, and `include_bytes!`",
);

declare_plrust_lint!(
    pub(crate) PLRUST_ENV_MACROS,
    "Disallow `env!`, and `option_env!`",
);

rustc_lint_defs::declare_lint_pass!(PlrustBuiltinMacros => [PLRUST_FILESYSTEM_MACROS]);

impl PlrustBuiltinMacros {
    fn lint_fs(&self, cx: &LateContext<'_>, sp: Span) {
        cx.lint(
            PLRUST_FILESYSTEM_MACROS,
            "the `include_str`, `include_bytes`, and `include` macros are forbidden in PL/Rust",
            |b| b.set_span(sp),
        );
    }
    fn lint_env(&self, cx: &LateContext<'_>, sp: Span) {
        cx.lint(
            PLRUST_ENV_MACROS,
            "the `env` and `option_env` macros are forbidden",
            |b| b.set_span(sp),
        );
    }
    fn check_span(&mut self, cx: &LateContext<'_>, span: Span) {
        let fs_diagnostic_items = [
            sym!(include_str_macro),
            sym!(include_bytes_macro),
            sym!(include_macro),
        ];
        if let Some((s, ..)) = utils::check_span_against_macro_diags(cx, span, &fs_diagnostic_items)
        {
            self.lint_fs(cx, s);
            if span != s {
                self.lint_fs(cx, span);
            }
        }
        let fs_def_paths: &[&[Symbol]] = &[
            &[sym!(core), sym!(macros), sym!(builtin), sym!(include)],
            &[sym!(core), sym!(macros), sym!(builtin), sym!(include_bytes)],
            &[sym!(core), sym!(macros), sym!(builtin), sym!(include_str)],
        ];
        if let Some((s, ..)) = utils::check_span_against_macro_def_paths(cx, span, &fs_def_paths) {
            self.lint_fs(cx, s);
            if span != s {
                self.lint_fs(cx, span);
            }
        }

        let env_diagnostic_items = [sym!(env_macro), sym!(option_env_macro)];
        if let Some((s, ..)) =
            utils::check_span_against_macro_diags(cx, span, &env_diagnostic_items)
        {
            self.lint_env(cx, span);
            if span != s {
                self.lint_env(cx, span);
            }
        }
        let env_def_paths: &[&[Symbol]] = &[
            &[sym!(core), sym!(macros), sym!(builtin), sym!(env)],
            &[sym!(core), sym!(macros), sym!(builtin), sym!(option_env)],
            &[sym!(core), sym!(macros), sym!(env)],
            &[sym!(core), sym!(macros), sym!(option_env)],
            &[sym!(core), sym!(env)],
            &[sym!(core), sym!(option_env)],
        ];
        if let Some((s, ..)) = utils::check_span_against_macro_def_paths(cx, span, &env_def_paths) {
            self.lint_env(cx, span);
            if span != s {
                self.lint_env(cx, span);
            }
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for PlrustBuiltinMacros {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &hir::Item) {
        self.check_span(cx, item.span)
    }
    fn check_stmt(&mut self, cx: &LateContext<'tcx>, stmt: &hir::Stmt) {
        self.check_span(cx, stmt.span)
    }
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &hir::Expr) {
        self.check_span(cx, expr.span)
    }
}
