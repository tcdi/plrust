#![feature(rustc_private)]

extern crate rustc_ast;
extern crate rustc_driver;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
#[macro_use]
extern crate rustc_session;
extern crate rustc_span;

use rustc_driver::plugin::Registry;
use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass, LintContext};

declare_lint!(
    pub(crate) EXTERN_BLOCKS,
    Allow,
    "Disallow extern blocks"
);

declare_lint_pass!(NoExternBlockPass => [EXTERN_BLOCKS]);

impl<'tcx> LateLintPass<'tcx> for NoExternBlockPass {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx hir::Item<'tcx>) {
        if let hir::ItemKind::ForeignMod { .. } = &item.kind {
            // TODO: Do we need to allow ones from macros from pgx?
            cx.lint(EXTERN_BLOCKS, |lint| {
                lint.build("`extern` blocks are forbidden in PL/Rust")
                    .set_span(item.span)
                    .emit()
            })
        }
    }
}

declare_lint!(
    pub(crate) LIFETIME_PARAMETERIZED_TRAITS,
    Allow,
    "Disallow lifetime parameterized traits"
);

declare_lint_pass!(LifetimeParamTraitPass => [LIFETIME_PARAMETERIZED_TRAITS]);

impl<'tcx> LateLintPass<'tcx> for LifetimeParamTraitPass {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx hir::Item<'tcx>) {
        if let hir::ItemKind::Trait(_is_auto, _unsafety, generics, ..) = &item.kind {
            for param in generics.params {
                if let hir::GenericParamKind::Lifetime { .. } = param.kind {
                    cx.lint(LIFETIME_PARAMETERIZED_TRAITS, |lint| {
                        lint.build(
                            "PL/Rust restricts the definition of lifetime parameterized traits.",
                        )
                        .set_span(item.span)
                        .emit()
                    })
                }
            }
        }
    }
}

#[no_mangle]
fn __rustc_plugin_registrar(reg: &mut Registry) {
    reg.lint_store
        .register_lints(&[&EXTERN_BLOCKS, &LIFETIME_PARAMETERIZED_TRAITS]);
    reg.lint_store
        .register_late_pass(move |_| Box::new(NoExternBlockPass));
    reg.lint_store
        .register_late_pass(move |_| Box::new(LifetimeParamTraitPass));
}
