use hir::{def::Res, def_id::DefId, Expr};
use once_cell::sync::Lazy;
use rustc_ast as ast;
use rustc_hir as hir;
use rustc_lint::{EarlyContext, EarlyLintPass, LateContext, LateLintPass, LintContext, LintStore};
use rustc_lint_defs::{declare_lint_pass, Lint, LintId};
use rustc_session::Session;
use rustc_span::hygiene::ExpnData;
use rustc_span::{Span, Symbol};

macro_rules! declare_plrust_lint {
    (
        $(#[$attr:meta])*
        $v:vis $NAME:ident,
        $desc:expr $(,)?
    ) => {
        rustc_lint_defs::declare_lint! (
            $(#[$attr])*
            $v $NAME,
            Allow,
            $desc,
            report_in_external_macro
        );
    };
}

/// `sym!(foo)` is shorthand for `Symbol::intern("foo")`
///
/// We *technically* could use `rustc_span::sym::$id` in some cases, but
/// rust-analyzer false-positives coupled with how often the set of symbols
/// changes... it feels like it could be a genuine maintenance issue to do that.
macro_rules! sym {
    ($id:ident) => {
        Symbol::intern(stringify!($id))
    };
}

// `sympath!(foo::bar)` is shorthand for `[sym!(foo), sym!(bar)]`
// macro_rules! sympath {
//     ($($component:ident)::+) => {
//         [$(sym!($component)),+]
//     };
// }

declare_plrust_lint!(
    pub(crate) PLRUST_EXTERN_BLOCKS,
    "Disallow extern blocks"
);

declare_lint_pass!(NoExternBlockPass => [PLRUST_EXTERN_BLOCKS]);

impl<'tcx> LateLintPass<'tcx> for NoExternBlockPass {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx hir::Item<'tcx>) {
        if let hir::ItemKind::ForeignMod { .. } = &item.kind {
            // TODO: Do we need to allow ones from macros from pgx?
            cx.lint(
                PLRUST_EXTERN_BLOCKS,
                "`extern` blocks are not allowed",
                |b| b.set_span(item.span),
            )
        }
    }
}

declare_plrust_lint!(
    pub(crate) PLRUST_STATIC_IMPLS,
    "Disallow impl blocks for types containing `'static` references"
);

declare_lint_pass!(PlrustStaticImpls => [PLRUST_STATIC_IMPLS]);

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
            | TyKind::Err
            // Doesn't exist
            | TyKind::Typeof(..)
            // Not strictly correct but we forbid this elsewhere anyway.
            | TyKind::BareFn(..)
            // TAIT stuff, still unstable at the moment, seems very hard to
            // prevent this for...
            | TyKind::OpaqueDef(..)
            | TyKind::Never => false,
            // Found one!
            TyKind::Rptr(Lifetime { res: Static, .. }, _) | TyKind::TraitObject(_, Lifetime { res: Static, .. }, _) => true,
            // Need to keep looking.
            TyKind::Rptr(_, MutTy { ty, .. })
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

declare_plrust_lint!(
    pub(crate) PLRUST_AUTOTRAIT_IMPLS,
    "Disallow impls of auto traits",
);

declare_lint_pass!(PlrustAutoTraitImpls => [PLRUST_AUTOTRAIT_IMPLS]);

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

declare_plrust_lint!(
    pub(crate) PLRUST_SUSPICIOUS_TRAIT_OBJECT,
    "Disallow suspicious generic use of trait objects",
);

declare_lint_pass!(PlrustSuspiciousTraitObject => [PLRUST_SUSPICIOUS_TRAIT_OBJECT]);

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
                        "trait objects in turbofish are forbidden",
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

declare_plrust_lint!(
    pub(crate) PLRUST_LIFETIME_PARAMETERIZED_TRAITS,
    "Disallow lifetime parameterized traits"
);

declare_lint_pass!(LifetimeParamTraitPass => [PLRUST_LIFETIME_PARAMETERIZED_TRAITS]);

impl<'tcx> LateLintPass<'tcx> for LifetimeParamTraitPass {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx hir::Item<'tcx>) {
        if let hir::ItemKind::Trait(_is_auto, _unsafety, generics, ..) = &item.kind {
            for param in generics.params {
                if let hir::GenericParamKind::Lifetime { .. } = param.kind {
                    cx.lint(
                        PLRUST_LIFETIME_PARAMETERIZED_TRAITS,
                        "Trait is parameterize by lifetime.",
                        |b| b.set_span(item.span),
                    )
                }
            }
        }
    }
}

declare_plrust_lint!(
    pub(crate) PLRUST_FILESYSTEM_MACROS,
    "Disallow `include_str!`, and `include_bytes!`",
);

declare_plrust_lint!(
    pub(crate) PLRUST_ENV_MACROS,
    "Disallow `env!`, and `option_env!`",
);

declare_lint_pass!(PlrustBuiltinMacros => [PLRUST_FILESYSTEM_MACROS]);
impl PlrustBuiltinMacros {
    fn lint_fs(&self, cx: &LateContext<'_>, sp: Span) {
        cx.lint(
            PLRUST_FILESYSTEM_MACROS,
            "the `include_str`, `include_bytes`, and `include` macros are forbidden",
            |b| b.set_span(sp),
        );
    }
    fn lint_env(&self, cx: &LateContext<'_>, sp: Span) {
        cx.lint(
            PLRUST_ENV_MACROS,
            "the `env` and `option_env` macros are forbidden",
            |b| b.set_span(sp),
        );
    }
    fn check_span(&mut self, cx: &LateContext<'_>, span: Span) {
        let fs_diagnostic_items = [
            sym!(include_str_macro),
            sym!(include_bytes_macro),
            sym!(include_macro),
        ];
        if let Some((s, ..)) = check_span_against_macro_diags(cx, span, &fs_diagnostic_items) {
            self.lint_fs(cx, s);
            return;
        }
        let fs_def_paths: &[&[Symbol]] = &[
            &[sym!(core), sym!(macros), sym!(builtin), sym!(include)],
            &[sym!(core), sym!(macros), sym!(builtin), sym!(include_bytes)],
            &[sym!(core), sym!(macros), sym!(builtin), sym!(include_str)],
        ];
        if let Some((s, ..)) = check_span_against_macro_def_paths(cx, span, &fs_def_paths) {
            self.lint_fs(cx, s);
            return;
        }

        let env_diagnostic_items = [sym!(env_macro), sym!(option_env_macro)];
        if let Some((s, ..)) = check_span_against_macro_diags(cx, span, &env_diagnostic_items) {
            self.lint_env(cx, s);
            return;
        }
        let env_def_paths: &[&[Symbol]] = &[
            &[sym!(core), sym!(macros), sym!(builtin), sym!(env)],
            &[sym!(core), sym!(macros), sym!(builtin), sym!(option_env)],
        ];
        if let Some((s, ..)) = check_span_against_macro_def_paths(cx, span, &env_def_paths) {
            self.lint_env(cx, s);
            return;
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for PlrustBuiltinMacros {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &hir::Item) {
        self.check_span(cx, item.span)
    }
    fn check_stmt(&mut self, cx: &LateContext<'tcx>, stmt: &hir::Stmt) {
        self.check_span(cx, stmt.span)
    }
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &hir::Expr) {
        self.check_span(cx, expr.span)
    }
}
fn outermost_expn_data(expn_data: ExpnData) -> ExpnData {
    if expn_data.call_site.from_expansion() {
        outermost_expn_data(expn_data.call_site.ctxt().outer_expn_data())
    } else {
        expn_data
    }
}

declare_plrust_lint!(
    pub(crate) PLRUST_STDIO,
    "Disallow functions like `io::{stdout, stderr, stdin}`",
);

declare_lint_pass!(PlrustPrintFunctions => [PLRUST_STDIO]);

impl<'tcx> LateLintPass<'tcx> for PlrustPrintFunctions {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &hir::Expr) {
        let paths: &[&[&str]] = &[
            &["std", "io", "stdio", "stdout"],
            &["std", "io", "stdio", "stderr"],
            &["std", "io", "stdio", "stdin"],
        ];
        for &path in paths {
            if does_expr_call_path(cx, expr, path) {
                cx.lint(
                    PLRUST_STDIO,
                    "the standard streams are forbidden, consider using `log!()` instead",
                    |b| b.set_span(expr.span),
                );
            }
        }
    }
}

declare_plrust_lint!(
    pub(crate) PLRUST_PRINT_MACROS,
    "Disallow `print!`, `println!`, `eprint!` and `eprintln!`",
);

declare_lint_pass!(PlrustPrintMacros => [PLRUST_PRINT_MACROS]);

impl PlrustPrintMacros {
    fn check_span(&self, cx: &LateContext<'_>, srcspan: Span) {
        let diagnostic_items = [
            sym!(print_macro),
            sym!(eprint_macro),
            sym!(println_macro),
            sym!(eprintln_macro),
            sym!(dbg_macro),
        ];
        if let Some((span, _which, _did)) =
            check_span_against_macro_diags(cx, srcspan, &diagnostic_items)
        {
            self.fire(cx, span);
        };
    }
    fn fire(&self, cx: &LateContext<'_>, span: Span) {
        cx.lint(
            PLRUST_PRINT_MACROS,
            "the printing macros are forbidden, consider using `log!()` instead",
            |b| b.set_span(span),
        );
    }
}
impl<'tcx> LateLintPass<'tcx> for PlrustPrintMacros {
    fn check_item(&mut self, cx: &LateContext<'tcx>, h: &hir::Item) {
        self.check_span(cx, h.span);
    }
    fn check_stmt(&mut self, cx: &LateContext<'tcx>, h: &hir::Stmt) {
        self.check_span(cx, h.span);
    }
    fn check_expr(&mut self, cx: &LateContext<'tcx>, h: &hir::Expr) {
        self.check_span(cx, h.span);
    }
}

fn check_span_against_macro_def_paths(
    cx: &LateContext<'_>,
    srcspan: Span,
    def_paths: &[&[Symbol]],
) -> Option<(Span, usize, DefId)> {
    let (which, defid) = iter_expn_data(srcspan).find_map(|expndata| {
        let Some(did) = expndata.macro_def_id else {
            return None;
        };
        let macro_def_path = cx.get_def_path(did);
        def_paths
            .iter()
            .position(|&defpath| defpath == macro_def_path.as_slice())
            .map(|pos| (pos, did))
    })?;
    let outermost_span = outermost_expn_data(srcspan.ctxt().outer_expn_data()).call_site;
    Some((outermost_span, which, defid))
}

// #[cfg(any())]
fn iter_expn_data(span: Span) -> impl Iterator<Item = ExpnData> {
    let mut next = Some(span.ctxt().outer_expn_data());
    std::iter::from_fn(move || {
        let curr = next.take()?;
        next = curr
            .call_site
            .from_expansion()
            .then_some(curr.call_site.ctxt().outer_expn_data());
        Some(curr)
    })
}
/// Note: this is error-prone!
fn outer_macro_call_matching<'tcx, F, T>(
    cx: &LateContext<'tcx>,
    span: Span,
    mut matches: F,
) -> Option<(Span, T, DefId)>
where
    F: FnMut(DefId, Option<Symbol>) -> Option<T>,
{
    let mut expn = span.ctxt().outer_expn_data();
    let mut found = None::<(T, DefId)>;
    loop {
        let parent = expn.call_site.ctxt().outer_expn_data();
        let Some(id) = parent.macro_def_id else {
            break;
        };
        let Some(thing) = matches(id, cx.tcx.get_diagnostic_name(id)) else {
            break;
        };
        expn = parent;
        found = Some((thing, id));
    }
    found.map(|(thing, defid)| (expn.call_site, thing, defid))
}

fn check_span_against_macro_diags(
    cx: &LateContext<'_>,
    span: Span,
    diag_syms: &[Symbol],
) -> Option<(Span, usize, DefId)> {
    outer_macro_call_matching(cx, span, |_did, diag_name| {
        let diag_name = diag_name?;
        diag_syms.iter().position(|&name| name == diag_name)
    })
}

declare_plrust_lint!(
    pub(crate) PLRUST_FN_POINTERS,
    "Disallow use of function pointers",
);

declare_lint_pass!(PlrustFnPointer => [PLRUST_FN_POINTERS]);

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
                            if match_def_path(cx, did, fn_trait_paths) {
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

declare_plrust_lint!(
    pub(crate) PLRUST_ASYNC,
    "Disallow use of async and await",
);

declare_lint_pass!(PlrustAsync => [PLRUST_ASYNC]);

impl EarlyLintPass for PlrustAsync {
    fn check_expr(&mut self, cx: &EarlyContext, expr: &ast::Expr) {
        if let ast::ExprKind::Async(..) | ast::ExprKind::Await(..) = &expr.kind {
            cx.lint(
                PLRUST_ASYNC,
                "Use of async/await is forbidden in PL/Rust",
                |b| b.set_span(expr.span),
            );
        }
    }
    fn check_fn(
        &mut self,
        cx: &EarlyContext,
        kind: ast::visit::FnKind<'_>,
        span: Span,
        _: ast::NodeId,
    ) {
        if let Some(h) = kind.header() {
            if h.asyncness.is_async() {
                cx.lint(
                    PLRUST_ASYNC,
                    "Use of async/await is forbidden in PL/Rust",
                    |b| b.set_span(span),
                );
            }
        }
    }
}

declare_plrust_lint!(
    pub(crate) PLRUST_EXTERNAL_MOD,
    "Disallow use of `mod blah;`",
);

declare_lint_pass!(PlrustExternalMod => [PLRUST_EXTERNAL_MOD]);

impl EarlyLintPass for PlrustExternalMod {
    fn check_item(&mut self, cx: &EarlyContext, item: &ast::Item) {
        match &item.kind {
            ast::ItemKind::Mod(_, ast::ModKind::Unloaded)
            | ast::ItemKind::Mod(_, ast::ModKind::Loaded(_, ast::Inline::No, _)) => {
                cx.lint(
                    PLRUST_EXTERNAL_MOD,
                    "Use of external modules is forbidden in PL/Rust",
                    |b| b.set_span(item.span),
                );
            }
            _ => {}
        }
    }
}

declare_plrust_lint!(
    pub(crate) PLRUST_LEAKY,
    "Disallow use of `{Box,Vec,String}::leak`, `mem::forget`, and similar functions",
);

declare_lint_pass!(PlrustLeaky => [PLRUST_LEAKY]);

impl<'tcx> LateLintPass<'tcx> for PlrustLeaky {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &hir::Expr) {
        let paths: &[&[&str]] = &[
            &["alloc", "boxed", "Box", "leak"],
            &["alloc", "vec", "Vec", "leak"],
            &["alloc", "string", "String", "leak"],
            &["core", "mem", "forget"],
        ];
        for &path in paths {
            if does_expr_call_path(cx, expr, path) {
                cx.lint(
                    PLRUST_LEAKY,
                    "Leaky functions are forbidden in PL/Rust",
                    |b| b.set_span(expr.span),
                );
            }
        }
    }
}
// Note: returns true if the expr is the path also.
fn does_expr_call_path(cx: &LateContext<'_>, expr: &Expr<'_>, segments: &[&str]) -> bool {
    path_res(cx, expr)
        .opt_def_id()
        .or_else(|| match &expr.kind {
            hir::ExprKind::MethodCall(..) => cx.typeck_results().type_dependent_def_id(expr.hir_id),
            _ => None,
        })
        .map_or(false, |id| match_def_path(cx, id, segments))
}

fn path_res(cx: &LateContext<'_>, ex: &Expr<'_>) -> Res {
    if let hir::ExprKind::Path(qpath) = &ex.kind {
        cx.qpath_res(qpath, ex.hir_id)
    } else {
        Res::Err
    }
}

fn match_def_path<'tcx>(cx: &LateContext<'tcx>, did: DefId, syms: &[&str]) -> bool {
    let path = cx.get_def_path(did);
    syms.iter()
        .map(|x| Symbol::intern(x))
        .eq(path.iter().copied())
}

// Used to force an ICE in our uitests. Only enabled if
// `PLRUSTC_INCLUDE_TEST_ONLY_LINTS` is enabled in the environment, which we do
// explicitly in the tests that need it.
declare_plrust_lint! {
    pub(crate) PLRUST_TEST_ONLY_FORCE_ICE,
    "This message should not appear in the output"
}

declare_lint_pass!(PlrustcForceIce => [PLRUST_TEST_ONLY_FORCE_ICE]);

impl EarlyLintPass for PlrustcForceIce {
    fn check_fn(
        &mut self,
        _: &EarlyContext<'_>,
        fn_kind: ast::visit::FnKind<'_>,
        _: Span,
        _: ast::NodeId,
    ) {
        use ast::visit::FnKind;
        const GIMME_ICE: &str = "plrustc_would_like_some_ice";
        if matches!(&fn_kind, FnKind::Fn(_, id, ..) if id.name.as_str() == GIMME_ICE) {
            panic!("Here is your ICE");
        }
    }
}

static INCLUDE_TEST_ONLY_LINTS: Lazy<bool> =
    Lazy::new(|| std::env::var("PLRUSTC_INCLUDE_TEST_ONLY_LINTS").is_ok());

static PLRUST_LINTS: Lazy<Vec<&'static Lint>> = Lazy::new(|| {
    let mut v = vec![
        PLRUST_ASYNC,
        PLRUST_AUTOTRAIT_IMPLS,
        PLRUST_STATIC_IMPLS,
        PLRUST_EXTERN_BLOCKS,
        PLRUST_EXTERNAL_MOD,
        PLRUST_FILESYSTEM_MACROS,
        PLRUST_ENV_MACROS,
        PLRUST_FN_POINTERS,
        PLRUST_LEAKY,
        PLRUST_LIFETIME_PARAMETERIZED_TRAITS,
        PLRUST_PRINT_MACROS,
        PLRUST_STDIO,
        PLRUST_SUSPICIOUS_TRAIT_OBJECT,
    ];
    if *INCLUDE_TEST_ONLY_LINTS {
        let test_only_lints = [PLRUST_TEST_ONLY_FORCE_ICE];
        v.extend(test_only_lints);
    }
    v
});

#[test]
fn check_lints() {
    for lint in &**PLRUST_LINTS {
        assert!(
            lint.name.starts_with("PLRUST_"),
            "lint `{}` doesn't follow lint naming convention",
            lint.name,
        );
        assert!(
            lint.report_in_external_macro,
            "lint `{}` should report in external macro expansion",
            lint.name,
        );
    }
}

pub fn register(store: &mut LintStore, _sess: &Session) {
    store.register_lints(&**PLRUST_LINTS);

    store.register_group(
        true,
        "plrust_lints",
        None,
        PLRUST_LINTS.iter().map(|&lint| LintId::of(lint)).collect(),
    );
    store.register_early_pass(move || Box::new(PlrustAsync));
    store.register_early_pass(move || Box::new(PlrustExternalMod));
    store.register_late_pass(move |_| Box::new(PlrustSuspiciousTraitObject));
    store.register_late_pass(move |_| Box::new(PlrustAutoTraitImpls));
    store.register_late_pass(move |_| Box::new(PlrustStaticImpls));
    store.register_late_pass(move |_| Box::new(PlrustFnPointer));
    store.register_late_pass(move |_| Box::new(PlrustLeaky));
    store.register_late_pass(move |_| Box::new(PlrustBuiltinMacros));
    store.register_late_pass(move |_| Box::new(PlrustPrintMacros));
    store.register_late_pass(move |_| Box::new(PlrustPrintFunctions));
    store.register_late_pass(move |_| Box::new(NoExternBlockPass));
    store.register_late_pass(move |_| Box::new(LifetimeParamTraitPass));

    if *INCLUDE_TEST_ONLY_LINTS {
        store.register_early_pass(move || Box::new(PlrustcForceIce));
    }
}
