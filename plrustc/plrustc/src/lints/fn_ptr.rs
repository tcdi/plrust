/*
Portions Copyright 2021-2025 Technology Concepts & Design, Inc.

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE file.
*/
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
                for poly_trait in *traits {
                    if super::utils::has_fn_trait(cx, poly_trait) {
                        cx.lint(
                            PLRUST_FN_POINTERS,
                            "Use of function trait objects is forbidden in PL/Rust",
                            |b| b.set_span(ty.span),
                        );
                    }
                }
            }
            _ => {}
        }
    }
}
