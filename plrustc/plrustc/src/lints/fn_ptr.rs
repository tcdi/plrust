use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass, LintContext};

declare_plrust_lint!(
    pub(crate) PLRUST_FN_POINTERS,
    "Disallow use of function pointers",
);

rustc_lint_defs::declare_lint_pass!(PlrustFnPointer => [PLRUST_FN_POINTERS]);

impl<'tcx> LateLintPass<'tcx> for PlrustFnPointer {
    fn check_ty(&mut self, cx: &LateContext<'tcx>, ty: &hir::Ty) {
        match &ty.kind {
            hir::TyKind::BareFn { .. } => {
                // TODO: ideally this would just be cases where they accept or
                // return nested references, however doing so is tricky, as it must
                // pierce through `&'a SomeStruct(&'b InternalRef)`.
                cx.lint(
                    PLRUST_FN_POINTERS,
                    "Use of function pointers is forbidden in PL/Rust",
                    |b| b.set_span(ty.span),
                );
            }
            hir::TyKind::TraitObject(traits, ..) => {
                for trayt in *traits {
                    if let Some(did) = trayt.trait_ref.path.res.opt_def_id() {
                        let fn_traits = [
                            &["core", "ops", "function", "Fn"],
                            &["core", "ops", "function", "FnMut"],
                            &["core", "ops", "function", "FnOnce"],
                        ];
                        for fn_trait_paths in fn_traits {
                            if super::utils::match_def_path(cx, did, fn_trait_paths) {
                                cx.lint(
                                    PLRUST_FN_POINTERS,
                                    "Use of function trait objects is forbidden in PL/Rust",
                                    |b| b.set_span(ty.span),
                                );
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
