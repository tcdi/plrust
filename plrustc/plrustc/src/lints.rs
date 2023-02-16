use once_cell::sync::Lazy;
use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass, LintContext, LintStore};
use rustc_lint_defs::{declare_lint, declare_lint_pass, Lint, LintId};
use rustc_session::Session;
use rustc_span::hygiene::ExpnData;

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

declare_lint!(
    pub(crate) PLRUST_FILESYSTEM_MACROS,
    Allow,
    "Disallow `include_str!`, and `include_bytes!`",
);

declare_lint_pass!(PlrustFilesystemMacros => [PLRUST_FILESYSTEM_MACROS]);

impl<'tcx> LateLintPass<'tcx> for PlrustFilesystemMacros {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &hir::Expr) {
        let expr_expn_data = expr.span.ctxt().outer_expn_data();
        let outermost_expn_data = outermost_expn_data(expr_expn_data);
        let Some(macro_def_id) = outermost_expn_data.macro_def_id else {
            return;
        };
        let Some(name) = cx.tcx.get_diagnostic_name(macro_def_id) else {
            return;
        };
        let diagnostic_items = ["include_str_macro", "include_bytes_macro"];
        if !diagnostic_items.contains(&name.as_str()) {
            return;
        }
        cx.lint(
            PLRUST_FILESYSTEM_MACROS,
            &format!("the `include_str` and `include_bytes` macros are forbidden"),
            |b| b.set_span(expr.span),
        );
    }
}

fn outermost_expn_data(expn_data: ExpnData) -> ExpnData {
    if expn_data.call_site.from_expansion() {
        outermost_expn_data(expn_data.call_site.ctxt().outer_expn_data())
    } else {
        expn_data
    }
}

declare_lint!(
    pub(crate) PLRUST_FN_POINTERS,
    Allow,
    "Disallow use of function pointers",
);

declare_lint_pass!(PlrustFnPointer => [PLRUST_FN_POINTERS]);

impl<'tcx> LateLintPass<'tcx> for PlrustFnPointer {
    fn check_ty(&mut self, cx: &LateContext<'tcx>, ty: &hir::Ty) {
        if let hir::TyKind::BareFn { .. } = &ty.kind {
            // TODO: ideally this would just be cases where they accept or
            // return nested references, however doing so is tricky, as it must
            // pierce through `&'a SomeStruct(&'b InternalRef)`.
            cx.lint(
                PLRUST_FN_POINTERS,
                "Use of function pointers is forbidden in PL/Rust",
                |b| b.set_span(ty.span),
            );
        }
    }
}

static PLRUST_LINTS: Lazy<Vec<&'static Lint>> = Lazy::new(|| {
    vec![
        PLRUST_EXTERN_BLOCKS,
        PLRUST_FILESYSTEM_MACROS,
        PLRUST_FN_POINTERS,
        PLRUST_LIFETIME_PARAMETERIZED_TRAITS,
    ]
});

pub fn register(store: &mut LintStore, _sess: &Session) {
    store.register_lints(&**PLRUST_LINTS);

    store.register_group(
        true,
        "plrust_lints",
        None,
        PLRUST_LINTS.iter().map(|&lint| LintId::of(lint)).collect(),
    );
    store.register_late_pass(move |_| Box::new(PlrustFnPointer));
    store.register_late_pass(move |_| Box::new(PlrustFilesystemMacros));
    store.register_late_pass(move |_| Box::new(NoExternBlockPass));
    store.register_late_pass(move |_| Box::new(LifetimeParamTraitPass));
}
