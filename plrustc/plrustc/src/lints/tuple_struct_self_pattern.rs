// Turn this into a no-op once 1.71.0 hits stable
// (https://github.com/rust-lang/rust/issues/111220)
use hir::def::Res;
use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty;

declare_plrust_lint!(
    pub(crate) PLRUST_TUPLE_STRUCT_SELF_PATTERN,
    "`Self` patterns for tuple structs",
);

rustc_lint_defs::declare_lint_pass!(TupleStructSelfPat => [PLRUST_TUPLE_STRUCT_SELF_PATTERN]);

impl<'tcx> LateLintPass<'tcx> for TupleStructSelfPat {
    fn check_pat(&mut self, cx: &LateContext<'tcx>, pat: &'tcx hir::Pat<'tcx>) {
        let hir::PatKind::TupleStruct(hir::QPath::Resolved(_, path), ..) = pat.kind else {
            return;
        };
        let Res::SelfCtor(ctor_did) = path.res else {
            return;
        };
        let o: Option<ty::EarlyBinder<ty::TraitRef>> = cx.tcx.impl_trait_ref(ctor_did);
        let Some(trait_ref) = o else {
            return;
        };
        let self_ty = trait_ref.0.self_ty();
        let ty::Adt(adt_def, _) = self_ty.kind() else {
            return;
        };
        let Some(ctor) = adt_def.non_enum_variant().ctor_def_id() else {
            return;
        };
        if !cx
            .tcx
            .visibility(ctor)
            .is_accessible_from(cx.tcx.parent_module(pat.hir_id).to_def_id(), cx.tcx)
        {
            cx.lint(
                PLRUST_TUPLE_STRUCT_SELF_PATTERN,
                "`Self` pattern on tuple struct used to access private field",
                |b| b.set_span(pat.span),
            );
        }
    }
}
