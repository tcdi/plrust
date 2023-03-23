use hir::{def::Res, def_id::DefId};
use rustc_hir as hir;
use rustc_lint::LateContext;
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
        rustc_span::Symbol::intern(stringify!($id))
    };
}

pub fn check_span_against_macro_def_paths(
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

pub fn outermost_expn_data(expn_data: ExpnData) -> ExpnData {
    if expn_data.call_site.from_expansion() {
        outermost_expn_data(expn_data.call_site.ctxt().outer_expn_data())
    } else {
        expn_data
    }
}

pub fn iter_expn_data(span: Span) -> impl Iterator<Item = ExpnData> {
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
pub fn outer_macro_call_matching<'tcx, F, T>(
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

pub fn check_span_against_macro_diags(
    cx: &LateContext<'_>,
    span: Span,
    diag_syms: &[Symbol],
) -> Option<(Span, usize, DefId)> {
    outer_macro_call_matching(cx, span, |_did, diag_name| {
        let diag_name = diag_name?;
        diag_syms.iter().position(|&name| name == diag_name)
    })
}

// Note: returns true if the expr is the path also.
pub fn does_expr_call_path(cx: &LateContext<'_>, expr: &hir::Expr<'_>, segments: &[&str]) -> bool {
    path_res(cx, expr)
        .opt_def_id()
        .or_else(|| match &expr.kind {
            hir::ExprKind::MethodCall(..) => cx.typeck_results().type_dependent_def_id(expr.hir_id),
            _ => None,
        })
        .map_or(false, |id| match_def_path(cx, id, segments))
}

pub fn path_res(cx: &LateContext<'_>, ex: &hir::Expr<'_>) -> Res {
    if let hir::ExprKind::Path(qpath) = &ex.kind {
        cx.qpath_res(qpath, ex.hir_id)
    } else {
        Res::Err
    }
}

pub fn match_def_path<'tcx>(cx: &LateContext<'tcx>, did: DefId, syms: &[&str]) -> bool {
    let path = cx.get_def_path(did);
    syms.iter()
        .map(|x| Symbol::intern(x))
        .eq(path.iter().copied())
}
