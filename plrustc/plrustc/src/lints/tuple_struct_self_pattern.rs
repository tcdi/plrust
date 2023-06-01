// Turn this into a no-op once 1.71.0 hits stable
// (https://github.com/rust-lang/rust/issues/111220)
use hir::def::Res;
use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass, LintContext};

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
        if let Res::SelfCtor(..) = path.res {
            // TODO: ideally we'd validate the visibility of the type (see
            // https://github.com/rust-lang/rust/commit/be44860ab94f9e469d6f02232d3064a1049c47ba),
            // but this is a pretty rare pattern, and doing so is pretty painful from here
            cx.lint(
                PLRUST_TUPLE_STRUCT_SELF_PATTERN,
                "`Self` pattern on tuple struct",
                |b| b.set_span(pat.span),
            )
        }
    }
}
