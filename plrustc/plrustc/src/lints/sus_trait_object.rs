use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass, LintContext};

declare_plrust_lint!(
    pub(crate) PLRUST_SUSPICIOUS_TRAIT_OBJECT,
    "Disallow suspicious generic use of trait objects",
);

rustc_lint_defs::declare_lint_pass!(PlrustSuspiciousTraitObject => [PLRUST_SUSPICIOUS_TRAIT_OBJECT]);

impl<'tcx> LateLintPass<'tcx> for PlrustSuspiciousTraitObject {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &hir::Expr) {
        let path_segments = match &expr.kind {
            hir::ExprKind::Path(hir::QPath::Resolved(_, path)) => path.segments,
            hir::ExprKind::Path(hir::QPath::TypeRelative(_, segment, ..))
            | hir::ExprKind::MethodCall(segment, ..) => std::slice::from_ref(*segment),
            // We're looking for expressions that (directly, since `check_expr`
            // will visit stuff that contains them through `Expr`) contain
            // paths, and there's nothing else.
            _ => return,
        };
        for segment in path_segments {
            let Some(args) = segment.args else {
                continue;
            };
            for arg in args.args {
                let hir::GenericArg::Type(ty) = arg else {
                    continue;
                };
                if let hir::TyKind::TraitObject(..) = &ty.kind {
                    cx.lint(
                        PLRUST_SUSPICIOUS_TRAIT_OBJECT,
                        "using trait objects in turbofish position is forbidden by PL/Rust",
                        |b| b.set_span(expr.span),
                    );
                }
            }
        }
    }

    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx hir::Item<'tcx>) {
        let generics = match &item.kind {
            hir::ItemKind::TyAlias(_, generics) => *generics,
            hir::ItemKind::Enum(_, generics) => *generics,
            hir::ItemKind::Struct(_, generics) => *generics,
            hir::ItemKind::Union(_, generics) => *generics,
            hir::ItemKind::Trait(_, _, generics, ..) => *generics,
            hir::ItemKind::Fn(_, generics, ..) => *generics,
            // Nothing else is stable and has `Generics`.
            _ => return,
        };
        for param in generics.params {
            let hir::GenericParamKind::Type { default: Some(ty), .. } = &param.kind else {
                continue;
            };
            if let hir::TyKind::TraitObject(..) = &ty.kind {
                cx.lint(
                    PLRUST_SUSPICIOUS_TRAIT_OBJECT,
                    "trait objects in generic defaults are forbidden",
                    |b| b.set_span(item.span),
                );
            }
        }
    }
}
