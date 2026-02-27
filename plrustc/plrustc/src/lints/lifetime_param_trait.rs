/*
Portions Copyright 2021-2025 Technology Concepts & Design, Inc.

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/
use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass, LintContext};

declare_plrust_lint!(
    pub(crate) PLRUST_LIFETIME_PARAMETERIZED_TRAITS,
    "Disallow lifetime parameterized traits"
);

rustc_lint_defs::declare_lint_pass!(LifetimeParamTraitPass => [PLRUST_LIFETIME_PARAMETERIZED_TRAITS]);

impl<'tcx> LateLintPass<'tcx> for LifetimeParamTraitPass {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx hir::Item<'tcx>) {
        let hir::ItemKind::Trait(_is_auto, _unsafety, generics, bounds, ..) = &item.kind else {
            return
        };
        let mut lifetime_count = 0usize;
        let mut nonlifetime_count = 0usize;
        for param in generics.params {
            if let hir::GenericParamKind::Lifetime { .. } = param.kind {
                lifetime_count += 1;
                // Don't allow params that come from a Binder
                if param.source != hir::GenericParamSource::Generics {
                    cx.lint(
                        PLRUST_LIFETIME_PARAMETERIZED_TRAITS,
                        "PL/Rust forbids declaring traits with lifetime parameters \
                        which aren't provided by generics",
                        |b| b.set_span(item.span),
                    );
                }
            } else {
                nonlifetime_count += 1;
            }
        }
        if lifetime_count == 0 {
            // No lifetime params, no problems (hopefully... In theory `Foo<&'a
            // Blah>` could be problematic, but forbidding that would break
            // quite a bit (so we should wait until we know its a problem), and
            // would have to be done at the use site anyway, not at the decl
            // like this lint)
            return;
        }
        // At this point we carve out a (suspiciously `serde`-shaped) hole for
        // common usage that should be non-problematic. In particular:
        //
        // - If there are no mixed lifetime and non-lifetime params
        // - If there is only one lifetime param.
        // - If the trait is (obviously) not object safe (we just check for
        //   direct `Sized` supertrait).
        // - If the trait has no non-sized supertraits.
        //
        // Things should(?) be fine...
        if nonlifetime_count > 0 {
            // mixed lifetime and non-lifetime params.
            cx.lint(
                PLRUST_LIFETIME_PARAMETERIZED_TRAITS,
                "PL/Rust forbids declaring traits with both generic lifetime parameters and non-lifetime parameters",
                |b| b.set_span(item.span),
            );
        }
        if lifetime_count > 1 {
            // More than one lifetime param.
            cx.lint(
                PLRUST_LIFETIME_PARAMETERIZED_TRAITS,
                "PL/Rust forbids declaring traits with both multiple lifetime parameters",
                |b| b.set_span(item.span),
            );
        }
        let mut is_sized = false;
        for bound in *bounds {
            match bound {
                hir::GenericBound::Outlives(..) => {
                    cx.lint(
                        PLRUST_LIFETIME_PARAMETERIZED_TRAITS,
                        "PL/Rust forbids declaring traits with lifetime parameters and outlives bounds",
                        |b| b.set_span(item.span),
                    );
                }
                hir::GenericBound::LangItemTrait(hir::LangItem::Sized, ..) => {
                    is_sized = true;
                }
                // Technically this is slightly too strict, but that's safe and
                // it significantly simplifies the lint.
                hir::GenericBound::LangItemTrait(..) | hir::GenericBound::Trait(..) => {
                    cx.lint(
                        PLRUST_LIFETIME_PARAMETERIZED_TRAITS,
                        "PL/Rust forbids declaring traits with lifetime parameters with supertraits \
                        (other than `Sized`, which is required)",
                        |b| b.set_span(item.span),
                    );
                }
            }
        }
        if !is_sized {
            cx.lint(
                PLRUST_LIFETIME_PARAMETERIZED_TRAITS,
                "PL/Rust forbids declaring object-safe traits with lifetime parameters",
                |b| b.set_span(item.span),
            );
        }
    }
}
