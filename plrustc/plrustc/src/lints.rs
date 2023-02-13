use once_cell::sync::Lazy;
use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass, LintContext, LintStore};
use rustc_lint_defs::{declare_lint, declare_lint_pass, Lint, LintId};
use rustc_session::Session;

declare_lint!(
    pub(crate) PLRUST_EXTERN_BLOCKS,
    Allow,
    "Disallow extern blocks"
);

declare_lint_pass!(NoExternBlockPass => [PLRUST_EXTERN_BLOCKS]);

impl<'tcx> LateLintPass<'tcx> for NoExternBlockPass {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx hir::Item<'tcx>) {
        if let hir::ItemKind::ForeignMod { .. } = &item.kind {
            // TODO: Do we need to allow ones from macros from pgx?
            cx.lint(
                PLRUST_EXTERN_BLOCKS,
                "`extern` blocks are not allowed",
                |b| b.set_span(item.span),
            )
        }
    }
}

declare_lint!(
    pub(crate) PLRUST_LIFETIME_PARAMETERIZED_TRAITS,
    Allow,
    "Disallow lifetime parameterized traits"
);

declare_lint_pass!(LifetimeParamTraitPass => [PLRUST_LIFETIME_PARAMETERIZED_TRAITS]);

impl<'tcx> LateLintPass<'tcx> for LifetimeParamTraitPass {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx hir::Item<'tcx>) {
        if let hir::ItemKind::Trait(_is_auto, _unsafety, generics, ..) = &item.kind {
            for param in generics.params {
                if let hir::GenericParamKind::Lifetime { .. } = param.kind {
                    cx.lint(
                        PLRUST_LIFETIME_PARAMETERIZED_TRAITS,
                        "Trait is parameterize by lifetime.",
                        |b| b.set_span(item.span),
                    )
                }
            }
        }
    }
}

static PLRUST_LINTS: Lazy<Vec<&'static Lint>> =
    Lazy::new(|| vec![PLRUST_EXTERN_BLOCKS, PLRUST_LIFETIME_PARAMETERIZED_TRAITS]);

pub fn register(store: &mut LintStore, _sess: &Session) {
    store.register_lints(&**PLRUST_LINTS);

    store.register_group(
        true,
        "plrust_lints",
        None,
        PLRUST_LINTS.iter().map(|&lint| LintId::of(lint)).collect(),
    );
    store.register_late_pass(move |_| Box::new(NoExternBlockPass));
    store.register_late_pass(move |_| Box::new(LifetimeParamTraitPass));
}
