/*
Portions Copyright 2021-2025 Technology Concepts & Design, Inc.

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE file.
*/
use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::Span;

declare_plrust_lint!(
    pub(crate) PLRUST_AUTOTRAIT_IMPLS,
    "Disallow impls of auto traits",
);

rustc_lint_defs::declare_lint_pass!(PlrustAutoTraitImpls => [PLRUST_AUTOTRAIT_IMPLS]);

impl<'tcx> LateLintPass<'tcx> for PlrustAutoTraitImpls {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx hir::Item<'tcx>) {
        fn trigger(cx: &LateContext<'_>, span: Span) {
            cx.lint(
                PLRUST_AUTOTRAIT_IMPLS,
                "explicit implementations of auto traits are forbidden in PL/Rust",
                |b| b.set_span(span),
            );
        }
        let hir::ItemKind::Impl(imp) = &item.kind else {
            return;
        };
        let Some(trait_) = &imp.of_trait else {
            return;
        };
        let Some(did) = trait_.trait_def_id() else {
            return;
        };
        // I think ideally we'd resolve `did` into the actual trait type and
        // check if it's audo, but I don't know how to do that here, so I'll
        // just check for UnwindSafe, RefUnwindSafe, and Unpin explicitly (these
        // are currently the only safe stable auto traits, I believe).
        //
        // The former two have diagnostic items, the latter doesn't but is a
        // lang item.
        if matches!(cx.tcx.lang_items().unpin_trait(), Some(unpin_did) if unpin_did == did) {
            trigger(cx, item.span);
        }

        if let Some(thing) = cx.tcx.get_diagnostic_name(did) {
            let name = thing.as_str();
            if name == "unwind_safe_trait" || name == "ref_unwind_safe_trait" {
                trigger(cx, item.span);
            }
        }
    }
}
