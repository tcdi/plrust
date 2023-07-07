use once_cell::sync::Lazy;
use rustc_lint::LintStore;
use rustc_lint_defs::{Lint, LintId};
use rustc_session::Session;

#[macro_use]
mod utils;

mod async_await;
mod autotrait_impls;
mod builtin_macros;
mod closure_trait_impl;
mod extern_blocks;
mod external_mod;
mod fn_ptr;
mod force_ice;
mod leaky;
mod lifetime_param_trait;
mod print_macros;
mod static_impls;
mod stdio;
mod sus_trait_object;
mod tuple_struct_self_pattern;

static INCLUDE_TEST_ONLY_LINTS: Lazy<bool> =
    Lazy::new(|| std::env::var("PLRUSTC_INCLUDE_TEST_ONLY_LINTS").is_ok());

static PLRUST_LINTS: Lazy<Vec<&'static Lint>> = Lazy::new(|| {
    let mut v = vec![
        async_await::PLRUST_ASYNC,
        autotrait_impls::PLRUST_AUTOTRAIT_IMPLS,
        closure_trait_impl::PLRUST_CLOSURE_TRAIT_IMPL,
        static_impls::PLRUST_STATIC_IMPLS,
        extern_blocks::PLRUST_EXTERN_BLOCKS,
        external_mod::PLRUST_EXTERNAL_MOD,
        builtin_macros::PLRUST_FILESYSTEM_MACROS,
        builtin_macros::PLRUST_ENV_MACROS,
        fn_ptr::PLRUST_FN_POINTERS,
        leaky::PLRUST_LEAKY,
        lifetime_param_trait::PLRUST_LIFETIME_PARAMETERIZED_TRAITS,
        print_macros::PLRUST_PRINT_MACROS,
        stdio::PLRUST_STDIO,
        sus_trait_object::PLRUST_SUSPICIOUS_TRAIT_OBJECT,
        tuple_struct_self_pattern::PLRUST_TUPLE_STRUCT_SELF_PATTERN,
    ];
    if *INCLUDE_TEST_ONLY_LINTS {
        let test_only_lints = [force_ice::PLRUST_TEST_ONLY_FORCE_ICE];
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
    store.register_early_pass(move || Box::new(async_await::PlrustAsync));
    store.register_early_pass(move || Box::new(external_mod::PlrustExternalMod));
    store.register_late_pass(move |_| Box::new(closure_trait_impl::PlrustClosureTraitImpl));
    store.register_late_pass(move |_| Box::new(sus_trait_object::PlrustSuspiciousTraitObject));
    store.register_late_pass(move |_| Box::new(autotrait_impls::PlrustAutoTraitImpls));
    store.register_late_pass(move |_| Box::new(static_impls::PlrustStaticImpls));
    store.register_late_pass(move |_| Box::new(fn_ptr::PlrustFnPointer));
    store.register_late_pass(move |_| Box::new(leaky::PlrustLeaky));
    store.register_late_pass(move |_| Box::new(builtin_macros::PlrustBuiltinMacros));
    store.register_late_pass(move |_| Box::new(print_macros::PlrustPrintMacros));
    store.register_late_pass(move |_| Box::new(stdio::PlrustPrintFunctions));
    store.register_late_pass(move |_| Box::new(extern_blocks::NoExternBlockPass));
    store.register_late_pass(move |_| Box::new(lifetime_param_trait::LifetimeParamTraitPass));
    store.register_late_pass(move |_| Box::new(tuple_struct_self_pattern::TupleStructSelfPat));

    if *INCLUDE_TEST_ONLY_LINTS {
        store.register_early_pass(move || Box::new(force_ice::PlrustcForceIce));
    }
}
