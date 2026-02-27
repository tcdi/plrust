/*
Portions Copyright 2021-2025 Technology Concepts & Design, Inc.

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/
//! Trait objects are [often inferred as 'static][trait-obj-default],
//! which can result, when combined with things like boxed closures,
//! in an incorrect lifetime also being inferred for the return type.
//! Allowing such a lifetime to become visible in Rust allows treating
//! `&'a mut str` as `&'static mut str`, with the "fun" that implies.
//!
//! Ideally we'd do this in such a way that it only forbids something that
//! allows projection through to the return type, for example
//! ```ignore (exposition-only)
//! trait Trait {
//!     type Assoc;
//! }
//! impl<R, F: Fn() -> R> Trait for F {
//!     type Assoc = R;
//! }
//! ```
//! allows `<_ as Trait>::Assoc` to get a functions return type. That said,
//! actually writing this lint has totally defeated me at the moment, so this is
//! good enough for now.
//!
//! For some intuition as to why this is tricky, consider cases like
//! ```ignore (exposition-only)
//! trait GivesAssoc<A> { type Assoc; }
//! impl<A> GivesAssoc<A> for A { type Assoc = A; }
//!
//! trait Child<T> where Self: GivesAssoc<T> {}
//! impl<R, F: Fn() -> R> Child<R> for F {}
//! ```
//! and similarly complicated variants. To figure this out you need to examine
//! not just the directly implemented trait, but also all traits that are
//! indirectly implemented via bounds as a result of the impl. This is possible,
//! but... difficult.
//!
//! [trait-obj-default]: https://doc.rust-lang.org/reference/lifetime-elision.html#default-trait-object-lifetimes
//!
use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass, LintContext};

declare_plrust_lint!(
    pub(crate) PLRUST_CLOSURE_TRAIT_IMPL,
    "Disallow trait impls which are generic over closure type",
);

rustc_lint_defs::declare_lint_pass!(PlrustClosureTraitImpl => [PLRUST_CLOSURE_TRAIT_IMPL]);

impl<'tcx> LateLintPass<'tcx> for PlrustClosureTraitImpl {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx hir::Item<'tcx>) {
        let hir::ItemKind::Impl(impl_item) = item.kind else {
            return
        };
        for pred in impl_item.generics.predicates {
            let hir::WherePredicate::BoundPredicate(bound_pred) = pred else {
                continue
            };
            // TODO: should we ignore cases where `bound_pred.bounded_ty` isn't
            // from one of `item.generics.params`?
            for bound in bound_pred.bounds {
                match bound {
                    hir::GenericBound::LangItemTrait(
                        hir::LangItem::Fn
                        | hir::LangItem::FnOnce
                        | hir::LangItem::FnMut
                        | hir::LangItem::FnPtrTrait,
                        ..,
                    ) => {
                        cx.lint(
                            PLRUST_CLOSURE_TRAIT_IMPL,
                            "trait impls bounded on function traits are forbidden in PL/Rust",
                            |b| b.set_span(bound_pred.span),
                        );
                    }
                    hir::GenericBound::LangItemTrait(..) => {
                        //  other stable builtin traits aren't useful for projecting a function's return type
                    }
                    hir::GenericBound::Trait(poly_trait, ..) => {
                        if super::utils::has_fn_trait(cx, poly_trait) {
                            cx.lint(
                                PLRUST_CLOSURE_TRAIT_IMPL,
                                "trait impls bounded on function traits are forbidden in PL/Rust",
                                |b| b.set_span(bound_pred.span),
                            );
                        }
                        // TODO: if that fails, do we need to
                        // try_normalize_erasing_regions and retry?
                    }
                    hir::GenericBound::Outlives(..) => {
                        // Lifetime generics are irrelevant for guards against type projection
                    }
                }
            }
        }
    }
}
