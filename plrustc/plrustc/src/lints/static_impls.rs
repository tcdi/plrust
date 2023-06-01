use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass, LintContext};

declare_plrust_lint!(
    pub(crate) PLRUST_STATIC_IMPLS,
    "Disallow impl blocks for types containing `'static` references"
);

rustc_lint_defs::declare_lint_pass!(PlrustStaticImpls => [PLRUST_STATIC_IMPLS]);

impl<'tcx> LateLintPass<'tcx> for PlrustStaticImpls {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx hir::Item<'tcx>) {
        let hir::ItemKind::Impl(imp) = &item.kind else {
            return;
        };
        if self.has_static(imp.self_ty) {
            cx.lint(
                PLRUST_STATIC_IMPLS,
                "`impl` blocks for types containing `'static` references are not allowed",
                |b| b.set_span(imp.self_ty.span),
            )
        }
    }
}

impl PlrustStaticImpls {
    /// This is pretty naïve and designed to match the specific patterns that
    /// trip up https://github.com/rust-lang/rust/issues/104005...
    ///
    /// Also, I feel like I should be able to use `hir::intravisit::walk_ty`
    /// here instead, but it doesn't seem to let be know the lifetime of refs,
    /// so... not sure...
    ///
    /// It's a method on `self` mostly to discourage use in other contexts,
    /// since it's probably wrong for them.
    fn has_static(&self, t: &hir::Ty) -> bool {
        use hir::{Lifetime, LifetimeName::Static, MutTy, TyKind};
        match &t.kind {
            TyKind::Infer
            | TyKind::Err(..)
            // Doesn't exist
            | TyKind::Typeof(..)
            // Not strictly correct but we forbid this elsewhere anyway.
            | TyKind::BareFn(..)
            // TAIT stuff, still unstable at the moment, seems very hard to
            // prevent this for...
            | TyKind::OpaqueDef(..)
            | TyKind::Never => false,
            // Found one!
            TyKind::Ref(Lifetime { res: Static, .. }, _) | TyKind::TraitObject(_, Lifetime { res: Static, .. }, _) => true,
            // Need to keep looking.
            TyKind::Ref(_, MutTy { ty, .. })
            | TyKind::Ptr(MutTy { ty, .. })
            | TyKind::Array(ty, _)
            | TyKind::Slice(ty) => self.has_static(*ty),

            TyKind::Tup(types) => types.iter().any(|t| self.has_static(t)),

            TyKind::TraitObject(polytrait, ..) => {
                polytrait.iter().any(|poly| {
                    self.segments_have_static(poly.trait_ref.path.segments)
                })
            }
            // Something like `Vec<T>` or `Option<T>`. Need to look inside...
            TyKind::Path(qpath) => {
                let segs = match qpath {
                    hir::QPath::Resolved(Some(maybe_ty), _) if self.has_static(maybe_ty) => return true,
                    hir::QPath::TypeRelative(t, _) if self.has_static(*t) => return true,
                    hir::QPath::LangItem(..) => return false,
                    hir::QPath::Resolved(_, path) => path.segments,
                    hir::QPath::TypeRelative(_, seg) => std::slice::from_ref(*seg),
                };
                self.segments_have_static(segs)
            }
        }
    }

    fn segments_have_static(&self, segs: &[hir::PathSegment]) -> bool {
        segs.iter().any(|seg| {
            seg.args().args.iter().any(|arg| match arg {
                hir::GenericArg::Lifetime(hir::Lifetime {
                    res: hir::LifetimeName::Static,
                    ..
                }) => true,
                hir::GenericArg::Type(t) => self.has_static(t),
                hir::GenericArg::Const(_) | hir::GenericArg::Infer(_)
                // Wasn't static
                | hir::GenericArg::Lifetime(_) => false,
            })
        })
    }
}
