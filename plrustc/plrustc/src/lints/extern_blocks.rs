use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass, LintContext};

declare_plrust_lint!(
    pub(crate) PLRUST_EXTERN_BLOCKS,
    "Disallow extern blocks"
);

rustc_lint_defs::declare_lint_pass!(NoExternBlockPass => [PLRUST_EXTERN_BLOCKS]);

impl<'tcx> LateLintPass<'tcx> for NoExternBlockPass {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx hir::Item<'tcx>) {
        if let hir::ItemKind::ForeignMod { .. } = &item.kind {
            // TODO: Do we need to allow ones from macros from pgrx?
            cx.lint(
                PLRUST_EXTERN_BLOCKS,
                "`extern` blocks are not allowed",
                |b| b.set_span(item.span),
            )
        }
    }
}
