/*
Portions Copyright 2021-2025 Technology Concepts & Design, Inc.

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE file.
*/
/*
Copyright 2021-2023 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

/*!
How to actually build and load a PL/Rust function
 */

/*
Consider opening the documentation like so:
```shell
cargo doc --no-deps --document-private-items --open
```
*/
use eyre::WrapErr;
use std::{path::Path, process::Output};

use pgrx::prelude::PgHeapTuple;
use pgrx::{pg_sys, PgBuiltInOids, PgOid};
use proc_macro2::TokenStream;
use quote::quote;

use crate::allow_list::{load_allowlist, AllowList, Error};
pub(crate) use build::FnBuild;
use crate_variant::CrateVariant;
pub(crate) use crating::FnCrating;
pub(crate) use loading::FnLoad;
pub(crate) use ready::FnReady;
pub(crate) use validate::FnValidate;
pub(crate) use verify::FnVerify;

use crate::prosrc::extract_source_and_capabilities_from_json;
use crate::target::CompilationTarget;
use crate::user_crate::capabilities::FunctionCapabilitySet;
use crate::user_crate::lint::LintSet;
use crate::PlRustError;

mod build;
pub(crate) mod capabilities;
mod cargo;
mod crate_variant;
mod crating;
pub(crate) mod lint;
mod loading;
mod ready;
mod validate;
mod verify;

/**
Finite state machine with "typestate" generic

This forces `UserCrate<P>` to follow the linear path:
```rust
FnCrating::try_from_$(inputs)_*
  -> FnCrating
  -> FnVerify
  -> FnBuild
  -> FnLoad
  -> FnReady
```
Rust's ownership types allow guaranteeing one-way consumption.
*/
pub(crate) struct UserCrate<P: CrateState>(P);

/**
Stages of PL/Rust compilation

Each CrateState implementation has some set of fn including equivalents to
```rust
fn new(args: A) -> Self;
fn next(self, args: N) -> Self::NextCrateState;
```

These are currently not part of CrateState as they are type-specific and
premature abstraction would be unwise.
*/
pub(crate) trait CrateState {}

impl UserCrate<FnCrating> {
    #[cfg(any(test, feature = "pg_test"))]
    #[tracing::instrument(level = "debug", skip_all)]
    pub(crate) fn generated_for_tests(
        generation_number: u64,
        db_oid: pg_sys::Oid,
        fn_oid: pg_sys::Oid,
        user_deps: toml::value::Table,
        user_code: syn::Block,
        variant: CrateVariant,
    ) -> Self {
        Self(FnCrating::for_tests(
            generation_number,
            db_oid,
            fn_oid,
            user_deps.into(),
            user_code,
            variant,
        ))
    }
    #[tracing::instrument(level = "debug", skip_all)]
    pub unsafe fn try_from_fn_oid(db_oid: pg_sys::Oid, fn_oid: pg_sys::Oid) -> eyre::Result<Self> {
        unsafe { FnCrating::try_from_fn_oid(db_oid, fn_oid).map(Self) }
    }
    #[tracing::instrument(level = "debug", skip_all)]
    #[allow(unused)] // used in tests
    pub fn lib_rs(&self) -> eyre::Result<(syn::File, LintSet)> {
        self.0.lib_rs()
    }
    #[tracing::instrument(level = "debug", skip_all)]
    #[allow(unused)] // used in tests
    pub fn cargo_toml(&self) -> eyre::Result<toml::value::Table> {
        self.0.cargo_toml()
    }
    /// Provision into a given folder and return the crate directory.
    #[tracing::instrument(level = "debug", skip_all)]
    pub fn provision(&self, parent_dir: &Path) -> eyre::Result<UserCrate<FnVerify>> {
        self.0.provision(parent_dir).map(UserCrate)
    }
}

impl UserCrate<FnVerify> {
    #[tracing::instrument(
        level = "debug",
        skip_all,
        fields(
            db_oid = %self.0.db_oid(),
            fn_oid = %self.0.fn_oid(),
            crate_dir = %self.0.crate_dir().display(),
            target_dir = tracing::field::display(target_dir.display()),
        ))]
    pub fn validate(self, target_dir: &Path) -> eyre::Result<(UserCrate<FnBuild>, Output)> {
        self.0
            .validate(target_dir)
            .map(|(state, output)| (UserCrate(state), output))
    }

    pub(crate) fn crate_dir(&self) -> &Path {
        self.0.crate_dir()
    }
}

impl UserCrate<FnBuild> {
    #[tracing::instrument(
        level = "debug",
        skip_all,
        fields(
            db_oid = %self.0.db_oid(),
            fn_oid = %self.0.fn_oid(),
            crate_dir = %self.0.crate_dir().display(),
            target_dir = tracing::field::display(target_dir.display()),
        ))]
    pub fn build(self, target_dir: &Path) -> eyre::Result<Vec<(UserCrate<FnLoad>, Output)>> {
        Ok(self
            .0
            .build(target_dir)?
            .into_iter()
            .map(|(state, output)| (UserCrate(state), output))
            .collect())
    }
}

impl UserCrate<FnLoad> {
    #[tracing::instrument(level = "debug")]
    pub(crate) fn built(
        generation_number: u64,
        db_oid: pg_sys::Oid,
        fn_oid: pg_sys::Oid,
        target: CompilationTarget,
        symbol: Option<String>,
        shared_object: Vec<u8>,
        lints: LintSet,
    ) -> Self {
        UserCrate(FnLoad::new(
            generation_number,
            db_oid,
            fn_oid,
            target,
            symbol,
            shared_object,
            lints,
        ))
    }

    #[tracing::instrument(level = "debug", skip_all)]
    pub fn into_inner(self) -> (CompilationTarget, Vec<u8>, LintSet) {
        self.0.into_inner()
    }

    #[tracing::instrument(level = "debug", skip_all)]
    pub unsafe fn validate(self) -> eyre::Result<UserCrate<FnValidate>> {
        unsafe { self.0.validate().map(UserCrate) }
    }
}

impl UserCrate<FnValidate> {
    #[tracing::instrument(level = "debug", skip_all)]
    pub(crate) unsafe fn load(self) -> eyre::Result<UserCrate<FnReady>> {
        unsafe { self.0.load().map(UserCrate) }
    }
}

impl UserCrate<FnReady> {
    #[tracing::instrument(level = "debug", skip_all)]
    pub unsafe fn evaluate(&self, fcinfo: pg_sys::FunctionCallInfo) -> pg_sys::Datum {
        unsafe { self.0.evaluate(fcinfo) }
    }

    pub(crate) fn close(self) -> eyre::Result<()> {
        self.0.close()
    }

    pub(crate) fn symbol_name(&self) -> &str {
        self.0.symbol_name()
    }

    #[inline]
    pub(crate) fn generation_number(&self) -> u64 {
        self.0.generation_number()
    }
}

#[tracing::instrument(level = "debug", skip_all, fields(type_oid = %type_oid.value()))]
pub(crate) fn oid_to_syn_type(
    type_oid: &PgOid,
    owned: bool,
    capabilities: &FunctionCapabilitySet,
) -> Result<syn::Type, PlRustError> {
    let array_type = unsafe { pg_sys::get_element_type(type_oid.value()) };

    let (base_oid, array) = if array_type != pg_sys::InvalidOid {
        (PgOid::from(array_type), true)
    } else {
        (type_oid.clone(), false)
    };

    let base_rust_type: TokenStream = match base_oid {
        PgOid::BuiltIn(builtin) => match builtin {
            PgBuiltInOids::ANYELEMENTOID => quote! { pgrx::AnyElement },
            PgBuiltInOids::BOOLOID => quote! { bool },
            PgBuiltInOids::BOXOID => quote! {pgrx::BOX },
            PgBuiltInOids::BYTEAOID if owned => quote! { Vec<u8> },
            PgBuiltInOids::BYTEAOID if !owned => quote! { &'a [u8] },
            PgBuiltInOids::CHAROID => quote! { u8 },
            PgBuiltInOids::CSTRINGOID if owned => quote! { std::ffi::CString },
            PgBuiltInOids::CSTRINGOID if !owned => quote! { &std::ffi::CStr },
            PgBuiltInOids::DATEOID => quote! { pgrx::Date },
            PgBuiltInOids::DATERANGEOID => quote! { Range<pgrx::Date> },
            PgBuiltInOids::FLOAT4OID => quote! { f32 },
            PgBuiltInOids::FLOAT8OID => quote! { f64 },
            // PgBuiltInOids::INETOID => quote! { Inet },
            PgBuiltInOids::INT2OID => quote! { i16 },
            PgBuiltInOids::INT4OID => quote! { i32 },
            PgBuiltInOids::INT4RANGEOID => quote! { Range<i32> },
            PgBuiltInOids::INT8OID => quote! { i64 },
            PgBuiltInOids::INT8RANGEOID => quote! { Range<i64> },
            PgBuiltInOids::INTERVALOID => quote! { pgrx::Interval },
            PgBuiltInOids::JSONBOID => quote! { pgrx::JsonB },
            PgBuiltInOids::JSONOID => quote! { pgrx::Json },
            PgBuiltInOids::POINTOID => quote! { pgrx::Point },
            PgBuiltInOids::NUMERICOID => quote! { pgrx::AnyNumeric },
            PgBuiltInOids::NUMRANGEOID => quote! { Range<pgrx::AnyNumeric> },
            PgBuiltInOids::OIDOID => quote! { pgrx::Oid },
            PgBuiltInOids::TEXTOID if owned => quote! { String },
            PgBuiltInOids::TEXTOID if !owned => quote! { &'a str },
            PgBuiltInOids::TIDOID => quote! { pg_sys::ItemPointerData },
            PgBuiltInOids::TIMEOID => quote! { pgrx::Time },
            PgBuiltInOids::TIMETZOID => quote! { pgrx::TimeWithTimeZone },
            PgBuiltInOids::TIMESTAMPOID => quote! { pgrx::Timestamp },
            PgBuiltInOids::TIMESTAMPTZOID => quote! { pgrx::TimestampWithTimeZone },
            PgBuiltInOids::TSRANGEOID => quote! { Range<pgrx::Timestamp> },
            PgBuiltInOids::TSTZRANGEOID => quote! { Range<pgrx::TimestampWithTimeZone> },
            PgBuiltInOids::UUIDOID => quote! { pgrx::Uuid },
            PgBuiltInOids::VARCHAROID => quote! { String },
            PgBuiltInOids::VOIDOID => quote! { () },
            PgBuiltInOids::RECORDOID => quote! { () },
            _ => return Err(PlRustError::NoOidToRustMapping(type_oid.value())),
        },
        PgOid::Custom(oid) => match PgHeapTuple::new_composite_type_by_oid(oid) {
            Ok(_) => {
                let oid_u32 = oid.as_u32();
                quote! { pgrx::composite_type!(#oid_u32) }
            }
            Err(_) => return Err(PlRustError::NoOidToRustMapping(oid)),
        },
        _ => return Err(PlRustError::NoOidToRustMapping(type_oid.value())),
    };

    let rust_type = if array {
        if owned {
            quote! { Vec<Option<#base_rust_type>> }
        } else if capabilities.has_zero_copy_arrays() {
            quote! { pgrx::Array<'a, #base_rust_type> }
        } else {
            // same as "owned"
            quote! { Vec<Option<#base_rust_type>> }
        }
    } else {
        base_rust_type
    };

    syn::parse2(rust_type.clone())
        .map_err(|e| PlRustError::ParsingRustMapping(type_oid.value(), rust_type.to_string(), e))
}

#[tracing::instrument(level = "debug", skip_all)]
fn parse_source_and_deps(
    code_and_deps: &str,
) -> eyre::Result<(syn::Block, toml::value::Table, FunctionCapabilitySet)> {
    enum Parse {
        Code,
        Deps,
    }

    // it's possible, especially via a `pg_restore` operation, that "code_and_deps" is actually
    // our JSON structure stored in `pg_proc.prosrc`.  We'll pass it to [`extract_source_and_capabilities_from_json`]
    // and let it figure out what to do.
    //
    // If it **is** our JSON structure, we only care about the `"src"` property.  That's all
    // [`maybe_extract_source_from_json`] returns anyways.  We ignore everything else that was there
    // and ultimately do a full compilation based on the current state of the Postgres database,
    // taking into account current GUC values and other parameters that may impact compilation.
    //
    // It's also possible "code_and_deps" is exactly that, given to us via a user-written
    // "CREATE OR REPLACE FUNCTION" statement.
    let (code_and_deps, capabilities) = extract_source_and_capabilities_from_json(code_and_deps);

    let mut deps_block = String::new();
    let mut code_block = String::from("{ ");
    let mut parse = Parse::Code;

    for line in code_and_deps.trim().split_inclusive('\n') {
        match line.trim() {
            "[dependencies]" => parse = Parse::Deps,
            "[code]" => parse = Parse::Code,
            _ => match parse {
                Parse::Code => code_block.push_str(line),
                Parse::Deps => deps_block.push_str(line),
            },
        }
    }

    code_block.push_str("\n}");

    let mut user_dependencies = validate_user_dependencies(deps_block)?;

    if crate::gucs::PLRUST_ALLOWED_DEPENDENCIES.get().is_some() {
        let allowlist = load_allowlist().wrap_err("Error loading dependency allow-list")?;
        user_dependencies = restrict_dependencies(user_dependencies, &allowlist)?;
    }

    let user_code: syn::Block =
        syn::parse_str(&code_block).map_err(PlRustError::ParsingCodeBlock)?;

    Ok((user_code, user_dependencies, capabilities))
}

#[tracing::instrument(level = "debug", skip_all)]
fn validate_user_dependencies(user_deps: String) -> eyre::Result<toml::value::Table> {
    let user_dependencies: toml::value::Table = toml::from_str(&user_deps)?;

    //
    // The validation process, such as it is, currently only requires that a dependency
    // entry's value either be a [toml::Value::String] or a [toml::Value::Table].  We
    // don't do any checking of the actual values here as they're essentially checked for
    // compatibility/usefulness when squaring up against the allow-list
    //
    // Additionally, if there is no allow-list, then we don't really care what's going on here --
    // the administrator has said it's fine for a function to YOLO, so who are we to judge?
    //

    for (dependency, val) in &user_dependencies {
        match val {
            // user dependencies in these general forms are allowed:
            //    name = "x.y.z"
            //    name = { version = "x.y.z", features = [ "a", "b", "c" ]
            toml::Value::String(_) | toml::Value::Table(_) => {
                // these are allowed
            }

            // everything else is unsupported
            _ => {
                return Err(eyre::eyre!("dependency `{}` is malformed", dependency));
            }
        }
    }

    Ok(user_dependencies)
}

#[derive(thiserror::Error, Debug, PartialEq)]
enum RestrictionError {
    #[error("`{0}` is not an allowed dependency")]
    DependencyNotAllowed(String),
    #[error("`{0}` does not specify a version")]
    VersionMissing(String),
    #[error("`{0}`'s version is not a String type")]
    NotAString(String),
    #[error("When specified, the dependency properties of `{1}` for `{0}` must be the same as the restricted set of `{2}`")]
    DependencyDeclarationMismatch(String, String, String),
    #[error("Dependency Error: {0}")]
    DependencyError(crate::allow_list::Error),
}

impl From<crate::allow_list::Error> for RestrictionError {
    fn from(value: Error) -> Self {
        RestrictionError::DependencyError(value)
    }
}

#[tracing::instrument(level = "debug", skip_all)]
fn restrict_dependencies(
    wanted: toml::value::Table,
    allowed: &AllowList,
) -> Result<toml::value::Table, RestrictionError> {
    fn extract_version<'a>(
        depname: &str,
        version_value: &'a toml::value::Value,
    ) -> Result<&'a str, RestrictionError> {
        match version_value {
            toml::value::Value::String(version) => Ok(version),
            toml::value::Value::Table(table) => table
                .get("version")
                .ok_or(RestrictionError::VersionMissing(depname.to_string()))?
                .as_str()
                .ok_or(RestrictionError::NotAString(depname.to_string())),
            _ => panic!("malformed dependency"),
        }
    }

    let mut actual = toml::value::Table::with_capacity(wanted.len());
    for (wanted_dep, wanted_value) in &wanted {
        let allowed_dep = allowed
            .get(wanted_dep)
            .ok_or_else(|| RestrictionError::DependencyNotAllowed(wanted_dep.clone()))?;
        let wanted_version = extract_version(wanted_dep, wanted_value)?;
        let used = allowed_dep.get_dependency_entry(wanted_version)?;

        // validate that the other properties that the user might have specified exactly match what
        // is in the allow-list
        if let Some(wanted_table) = wanted_value.as_table() {
            if wanted_table.len() > 1 {
                let mut wanted = wanted_table.clone();
                let mut used = used.as_table().unwrap().clone();

                // don't compare the version values
                wanted.remove("version");
                used.remove("version");

                if used != wanted {
                    return Err(RestrictionError::DependencyDeclarationMismatch(
                        wanted_dep.clone(),
                        toml::to_string(&wanted).unwrap().trim().replace('\n', ", "),
                        toml::to_string(&used).unwrap().trim().replace('\n', ", "),
                    ));
                }
            }
        }

        actual.insert(wanted_dep.clone(), used);
    }

    Ok(actual)
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use crate::pgproc::ProArgMode;
    use pgrx::*;
    use proc_macro2::{Ident, Span};
    use quote::quote;
    use syn::parse_quote;

    use crate::user_crate::crating::cargo_toml_template;
    use crate::user_crate::*;

    #[rustfmt::skip]
    #[test]
    fn test_restrict_dependencies() -> eyre::Result<()> {
        use crate::allow_list::{parse_allowlist, Error};
        use crate::user_crate::{restrict_dependencies, RestrictionError};
        use toml::toml;

        const TOML: &str = r#"
a = [ "=1.2.3", "=3.0", ">=6.0.0, <=10", { version = "=2.4.5", features = [ "x", "y", "z" ] }, "*", ">=1.0.0, <5.0.0",">=1.0.0, <2.0.0", ">=2, <=4", "=2.99.99" ]
b = "*"
c = "=1.2.3"
d = { version = "=3.4.5", features = [ "x", "y", "z" ] }
e = ">=0.8, <0.9"
    "#;

        let allowed = parse_allowlist(TOML)?;

        let restricted = restrict_dependencies(toml! { a = "3.0" }, &allowed)?;
        assert_eq!(toml! { a = { version = "=3.0" } }, restricted);

        let restricted = restrict_dependencies(toml! { a = { version = "2.4.5", features = [ "q", "r", "p" ], default-features = false } }, &allowed);
        if !matches!(restricted, Err(RestrictionError::DependencyDeclarationMismatch(..))) {
            panic!("got valid restricted table when we shouldn't have");
        }

        let restricted = restrict_dependencies(toml! { a = { version = "2.4.5", features = [ "x", "y", "z" ] } }, &allowed)?;
        assert_eq!(toml! { a = { version = "=2.4.5", features = [ "x", "y", "z" ] } }, restricted);

        let restricted = restrict_dependencies(toml! { a = { version = "2.4.5" } }, &allowed)?;
        assert_eq!(toml! { a = { version = "=2.4.5", features = [ "x", "y", "z" ] } }, restricted);

        let restricted = restrict_dependencies(toml! { a = { version = "6.7.8" } }, &allowed)?;
        assert_eq!(toml! { a = { version = "=6.7.8" } }, restricted);

        let restricted = restrict_dependencies(toml! { a = { version = "=6.7.8" } }, &allowed)?;
        assert_eq!(toml! { a = { version = "=6.7.8" } }, restricted);

        let restricted = restrict_dependencies(toml! { a = "*" }, &allowed)?;
        assert_eq!(toml! { a = { version = ">=6.0.0, <=10" } }, restricted);

        let restricted = restrict_dependencies(toml! { b = "=1.2.3" } , &allowed)?;
        assert_eq!(toml! { b = { version = "=1.2.3" } }, restricted);

        let restricted = restrict_dependencies(toml! { b = "42" } , &allowed)?;
        assert_eq!(toml! { b = { version = "^42" } }, restricted);

        let restricted = restrict_dependencies(toml! { c = "1.2.3" } , &allowed)?;
        assert_eq!(toml! { c = { version = "=1.2.3" } }, restricted);

        let restricted = restrict_dependencies(toml! { c = "*" } , &allowed)?;
        assert_eq!(toml! { c = { version = "=1.2.3" } }, restricted);

        let restricted = restrict_dependencies(toml! { c = "1" } , &allowed)?;
        assert_eq!(toml! { c = { version = "=1.2.3" } }, restricted);

        let restricted = restrict_dependencies(toml! { c = "99" } , &allowed);
        assert_eq!(restricted, Err(RestrictionError::DependencyError(Error::VersionNotPermitted("99".to_string()))));

        let restricted = restrict_dependencies(toml! { e = "0.8" } , &allowed)?;
        assert_eq!(toml! { e = { version = ">=0.8, <0.9" } }, restricted);

        let restricted = restrict_dependencies(toml! { e = "0.9" } , &allowed);
        assert_eq!(restricted, Err(RestrictionError::DependencyError(Error::VersionNotPermitted("0.9".to_string()))));

        Ok(())
    }

    #[pg_test]
    fn full_workflow() {
        fn wrapped() -> eyre::Result<()> {
            let generation_number = 0;
            let fn_oid = pg_sys::Oid::INVALID;
            let db_oid = unsafe { pg_sys::MyDatabaseId };
            let target_dir = crate::gucs::work_dir();

            let variant = {
                let argnames = vec![Ident::new("arg0", Span::call_site())];
                let argtypes = vec![pg_sys::TEXTOID];
                let argmodes = vec![ProArgMode::In];
                let return_oid = PgOid::from(PgBuiltInOids::TEXTOID.value());
                let is_strict = true;
                let return_set = false;
                CrateVariant::function(
                    argnames,
                    argtypes,
                    argmodes,
                    return_oid,
                    return_set,
                    is_strict,
                    FunctionCapabilitySet::default(),
                )?
            };
            let user_deps = toml::value::Table::default();
            let user_code = syn::parse2(quote! {
                { Ok(Some(arg0.to_string())) }
            })?;

            let generated = UserCrate::generated_for_tests(
                generation_number,
                db_oid,
                fn_oid,
                user_deps,
                user_code,
                variant,
            );
            let symbol_name = crate::plrust::symbol_name(db_oid, fn_oid);
            let symbol_ident =
                proc_macro2::Ident::new(&symbol_name, proc_macro2::Span::call_site());

            let (generated_lib_rs, lints) = generated.lib_rs()?;
            let imports = crate::user_crate::crating::shared_imports();
            let bare_fn: syn::ItemFn = syn::parse2(quote! {
                fn #symbol_ident<'a>(arg0: &'a str) -> ::std::result::Result<Option<String>, Box<dyn std::error::Error + Send + Sync + 'static>> {
                    Ok(Some(arg0.to_string()))
                }
            })?;
            let fixture_lib_rs = parse_quote! {
                #![deny(unsafe_op_in_unsafe_fn)]
                pub mod opened {
                    #imports

                    #[allow(unused_lifetimes)]
                    #[pg_extern]
                    #bare_fn
                }

                #[deny(unknown_lints)]
                mod forbidden {
                    #lints
                    #imports

                    #[allow(unused_lifetimes)]
                    #bare_fn
                }
            };
            assert_eq!(
                prettyplease::unparse(&generated_lib_rs),
                prettyplease::unparse(&fixture_lib_rs),
                "Generated `lib.rs` differs from test (after formatting)",
            );

            let generated_cargo_toml = generated.cargo_toml()?;
            let version_feature = format!("pgrx/pg{}", pgrx::pg_sys::get_pg_major_version_num());
            let crate_name = crate::plrust::crate_name(db_oid, fn_oid, generation_number);
            let fixture_cargo_toml = cargo_toml_template(&crate_name, &version_feature);

            assert_eq!(
                toml::to_string(&generated_cargo_toml)?,
                toml::to_string(&fixture_cargo_toml)?,
                "Generated `Cargo.toml` differs from test (after formatting)",
            );

            let provisioned = generated.provision(&target_dir)?;

            let (validated, _output) = provisioned.validate(&target_dir)?;

            for (built, _output) in validated.build(&target_dir)? {
                // Without an fcinfo, we can't call this.
                let validated = unsafe { built.validate()? };
                let _loaded = unsafe { validated.load()? };
            }

            Ok(())
        }
        wrapped().unwrap()
    }
}
