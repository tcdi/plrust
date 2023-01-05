/*!
How to actually build and load a PL/Rust function
*/

/*
Consider opening the documentation like so:
```shell
cargo doc --no-deps --document-private-items --open
```
*/
mod build;
mod crate_variant;
mod crating;
mod loading;
mod ready;
mod target;
mod verify;

pub(crate) use build::FnBuild;
use crate_variant::CrateVariant;
pub(crate) use crating::FnCrating;
pub(crate) use loading::FnLoad;
pub(crate) use ready::FnReady;
pub(crate) use verify::FnVerify;

use crate::PlRustError;
use pgx::{pg_sys, PgBuiltInOids, PgOid};
use proc_macro2::TokenStream;
use quote::quote;
use semver;
use std::{
    path::{Path, PathBuf},
    process::Output,
};

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
        pg_proc_xmin: pg_sys::TransactionId,
        db_oid: pg_sys::Oid,
        fn_oid: pg_sys::Oid,
        user_deps: toml::value::Table,
        user_code: syn::Block,
        variant: CrateVariant,
    ) -> Self {
        Self(FnCrating::for_tests(
            pg_proc_xmin,
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
    pub fn lib_rs(&self) -> eyre::Result<syn::File> {
        let lib_rs = self.0.lib_rs()?;
        Ok(lib_rs)
    }
    #[tracing::instrument(level = "debug", skip_all)]
    #[allow(unused)] // used in tests
    pub fn cargo_toml(&self) -> eyre::Result<toml::value::Table> {
        self.0.cargo_toml()
    }
    /// Provision into a given folder and return the crate directory.
    #[tracing::instrument(level = "debug", skip_all, fields(db_oid = %self.0.db_oid(), fn_oid = %self.0.fn_oid()))]
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
    pub fn validate(
        self,
        pg_config: PathBuf,
        target_dir: &Path,
    ) -> eyre::Result<(UserCrate<FnBuild>, Output)> {
        self.0
            .validate(pg_config, target_dir)
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
    pub fn build(self, target_dir: &Path) -> eyre::Result<(UserCrate<FnLoad>, Output)> {
        self.0
            .build(target_dir)
            .map(|(state, output)| (UserCrate(state), output))
    }
}

impl UserCrate<FnLoad> {
    #[tracing::instrument(level = "debug")]
    pub(crate) fn built(
        pg_proc_xmin: pg_sys::TransactionId,
        db_oid: pg_sys::Oid,
        fn_oid: pg_sys::Oid,
        shared_object: PathBuf,
    ) -> Self {
        UserCrate(FnLoad::new(
            pg_proc_xmin,
            db_oid,
            fn_oid,
            shared_object.to_path_buf(),
        ))
    }
    #[tracing::instrument(level = "debug", skip_all, fields(db_oid = %self.0.db_oid(), fn_oid = %self.0.fn_oid()))]
    pub fn shared_object(&self) -> &Path {
        self.0.shared_object()
    }
    #[tracing::instrument(level = "debug", skip_all, fields(db_oid = %self.0.db_oid(), fn_oid = %self.0.fn_oid()))]
    pub unsafe fn load(self) -> eyre::Result<UserCrate<FnReady>> {
        unsafe { self.0.load().map(UserCrate) }
    }
}

impl UserCrate<FnReady> {
    #[tracing::instrument(level = "debug", skip_all, fields(db_oid = %self.db_oid(), fn_oid = %self.fn_oid()))]
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
    pub(crate) fn xmin(&self) -> pg_sys::TransactionId {
        self.0.xmin()
    }

    pub(crate) fn fn_oid(&self) -> pg_sys::Oid {
        self.0.fn_oid()
    }

    pub(crate) fn db_oid(&self) -> pg_sys::Oid {
        self.0.db_oid()
    }

    pub(crate) fn shared_object(&self) -> &Path {
        self.0.shared_object()
    }
}

#[tracing::instrument(level = "debug", skip_all, fields(type_oid = type_oid.value()))]
pub(crate) fn oid_to_syn_type(type_oid: &PgOid, owned: bool) -> Result<syn::Type, PlRustError> {
    let array_type = unsafe { pg_sys::get_element_type(type_oid.value()) };

    let (base_oid, array) = if array_type != pg_sys::InvalidOid {
        (PgOid::from(array_type), true)
    } else {
        (type_oid.clone(), false)
    };

    let base_rust_type: TokenStream = match base_oid {
        PgOid::BuiltIn(builtin) => match builtin {
            PgBuiltInOids::ANYELEMENTOID => quote! { AnyElement },
            PgBuiltInOids::BOOLOID => quote! { bool },
            PgBuiltInOids::BYTEAOID if owned => quote! { Vec<Option<[u8]>> },
            PgBuiltInOids::BYTEAOID if !owned => quote! { &[u8] },
            PgBuiltInOids::CHAROID => quote! { u8 },
            PgBuiltInOids::CSTRINGOID => quote! { std::ffi::CStr },
            PgBuiltInOids::FLOAT4OID => quote! { f32 },
            PgBuiltInOids::FLOAT8OID => quote! { f64 },
            PgBuiltInOids::INETOID => quote! { Inet },
            PgBuiltInOids::INT2OID => quote! { i16 },
            PgBuiltInOids::INT4OID => quote! { i32 },
            PgBuiltInOids::INT8OID => quote! { i64 },
            PgBuiltInOids::JSONBOID => quote! { JsonB },
            PgBuiltInOids::JSONOID => quote! { Json },
            PgBuiltInOids::NUMERICOID => quote! { AnyNumeric },
            PgBuiltInOids::OIDOID => quote! { pg_sys::Oid },
            PgBuiltInOids::TEXTOID if owned => quote! { String },
            PgBuiltInOids::TEXTOID if !owned => quote! { &str },
            PgBuiltInOids::TIDOID => quote! { pg_sys::ItemPointer },
            PgBuiltInOids::VARCHAROID => quote! { String },
            PgBuiltInOids::VOIDOID => quote! { () },
            _ => return Err(PlRustError::NoOidToRustMapping(type_oid.value())),
        },
        _ => return Err(PlRustError::NoOidToRustMapping(type_oid.value())),
    };

    let rust_type = if array {
        quote! { Vec<Option<#base_rust_type>> }
    } else {
        base_rust_type
    };

    syn::parse2(rust_type.clone())
        .map_err(|e| PlRustError::ParsingRustMapping(type_oid.value(), rust_type.to_string(), e))
}

#[tracing::instrument(level = "debug", skip_all)]
fn parse_source_and_deps(code_and_deps: &str) -> eyre::Result<(syn::Block, toml::value::Table)> {
    enum Parse {
        Code,
        Deps,
    }
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

    code_block.push_str(" }");

    let user_dependencies = check_user_dependencies(deps_block)?;

    let user_code: syn::Block =
        syn::parse_str(&code_block).map_err(PlRustError::ParsingCodeBlock)?;

    Ok((user_code, user_dependencies))
}

#[tracing::instrument(level = "debug", skip_all)]
fn check_user_dependencies(user_deps: String) -> eyre::Result<toml::value::Table> {
    let user_dependencies: toml::value::Table = toml::from_str(&user_deps)?;

    for (dependency, val) in &user_dependencies {
        match val {
            toml::Value::String(_) => {
                // No-op, we currently only support dependencies in the format
                // foo = "1.0.0"
            }
            _ => {
                return Err(eyre::eyre!(
                    "dependency {} with values {:?} is malformatted. Only strings are supported",
                    dependency,
                    val
                ));
            }
        }
    }

    check_dependencies_against_allowed(&user_dependencies)?;
    Ok(user_dependencies)
}

#[tracing::instrument(level = "debug", skip_all)]
fn check_dependencies_against_allowed(dependencies: &toml::value::Table) -> eyre::Result<()> {
    if matches!(crate::gucs::PLRUST_ALLOWED_DEPENDENCIES.get(), None) {
        return Ok(());
    }

    let allowed_deps = &*crate::gucs::PLRUST_ALLOWED_DEPENDENCIES_CONTENTS;
    let mut unsupported_deps = std::vec::Vec::new();

    for (dep, val) in dependencies {
        if !allowed_deps.contains_key(dep) {
            unsupported_deps.push(format!("{} = {}", dep, val.to_string()));
            continue;
        }

        match val {
            toml::Value::String(ver) => {
                let req = semver::VersionReq::parse(ver.as_str()).unwrap();

                // Check if the allowed dependency is of format String or toml::Table
                // foo = "1.0.0" vs foo = { version = "1.0.0", features = ["full", "boo"], test = ["single"]}
                match allowed_deps.get(dep).unwrap() {
                    toml::Value::String(allowed_deps_ver) => {
                        if !req.matches(&semver::Version::parse(allowed_deps_ver)?) {
                            unsupported_deps.push(format!("{} = {}", dep, val.to_string()));
                        }
                    }
                    toml::Value::Table(allowed_deps_vals) => {
                        if !req.matches(&semver::Version::parse(
                            &allowed_deps_vals.get("version").unwrap().as_str().unwrap(),
                        )?) {
                            unsupported_deps.push(format!("{} = {}", dep, val.to_string()));
                        }
                    }
                    _ => {
                        return Err(eyre::eyre!(
                            "{} contains an unsupported toml format",
                            crate::gucs::PLRUST_ALLOWED_DEPENDENCIES.get().unwrap()
                        ));
                    }
                }
            }
            _ => {
                return Err(eyre::eyre!(
                    "dependency {} with values {:?} is malformatted. Only strings are supported",
                    dep,
                    val
                ));
            }
        }
    }

    if !unsupported_deps.is_empty() {
        return Err(eyre::eyre!(
            "The following dependencies are unsupported {:?}. The configured PL/Rust only supports {:?}",
            unsupported_deps,
            allowed_deps
        ));
    }

    Ok(())
}

#[cfg(any(test, feature = "pg_test"))]
#[pgx::pg_schema]
mod tests {
    use pgx::*;

    use crate::user_crate::*;
    use quote::quote;
    use syn::parse_quote;
    use toml::toml;

    #[pg_test]
    fn full_workflow() {
        fn wrapped() -> eyre::Result<()> {
            let pg_proc_oid = 0 as pg_sys::TransactionId;
            let fn_oid = 0 as pg_sys::Oid;
            let db_oid = 1 as pg_sys::Oid;
            let target_dir = crate::gucs::work_dir();
            let pg_config = PathBuf::from(crate::gucs::pg_config());

            let variant = {
                let argument_oids_and_names =
                    vec![(PgOid::from(PgBuiltInOids::TEXTOID.value()), None)];
                let return_oid = PgOid::from(PgBuiltInOids::TEXTOID.value());
                let is_strict = true;
                let return_set = false;
                CrateVariant::function(argument_oids_and_names, return_oid, return_set, is_strict)?
            };
            let user_deps = toml::value::Table::default();
            let user_code = syn::parse2(quote! {
                { Some(arg0.to_string()) }
            })?;

            let generated = UserCrate::generated_for_tests(
                pg_proc_oid,
                db_oid,
                fn_oid,
                user_deps,
                user_code,
                variant,
            );
            let crate_name = crate::plrust::crate_name(db_oid, fn_oid);
            #[cfg(any(
                all(target_os = "macos", target_arch = "x86_64"),
                feature = "force_enable_x86_64_darwin_generations"
            ))]
            let crate_name = {
                let mut crate_name = crate_name;
                let (latest, _path) =
                    crate::generation::latest_generation(&crate_name, true).unwrap_or_default();

                crate_name.push_str(&format!("_{}", latest));
                crate_name
            };
            let symbol_ident = proc_macro2::Ident::new(&crate_name, proc_macro2::Span::call_site());

            let generated_lib_rs = generated.lib_rs()?;
            let imports = crate::user_crate::crating::shared_imports();
            let bare_fn: syn::ItemFn = syn::parse2(quote! {
                fn #symbol_ident(arg0: &str) -> Option<String> {
                    Some(arg0.to_string())
                }
            })?;
            let fixture_lib_rs = parse_quote! {
                #![deny(unsafe_op_in_unsafe_fn)]
                pub mod opened {
                    #imports

                    #[pg_extern]
                    #bare_fn
                }

                mod forbidden {
                    #![forbid(unsafe_code)]
                    #imports

                    #bare_fn
                }
            };
            assert_eq!(
                prettyplease::unparse(&generated_lib_rs),
                prettyplease::unparse(&fixture_lib_rs),
                "Generated `lib.rs` differs from test (after formatting)",
            );

            let generated_cargo_toml = generated.cargo_toml()?;
            let version_feature = format!("pgx/pg{}", pgx::pg_sys::get_pg_major_version_num());
            let fixture_cargo_toml = toml! {
                [package]
                edition = "2021"
                name = crate_name
                version = "0.0.0"

                [features]
                default = [version_feature]

                [lib]
                crate-type = ["cdylib"]

                [dependencies]
                pgx = { git = "https://github.com/tcdi/pgx", branch = "develop" }
                // pallocator = { version = "0.1.0", git = "https://github.com/tcdi/postgrestd", branch = "1.61" }
                /* User deps added here */

                [profile.release]
                debug-assertions = true
                codegen-units = 1_usize
                lto = "fat"
                opt-level = 3_usize
                panic = "unwind"
            };
            assert_eq!(
                toml::to_string(&generated_cargo_toml)?,
                toml::to_string(&fixture_cargo_toml)?,
                "Generated `Cargo.toml` differs from test (after formatting)",
            );

            let provisioned = generated.provision(&target_dir)?;

            let (validated, _output) = provisioned.validate(pg_config, &target_dir)?;

            let (built, _output) = validated.build(&target_dir)?;

            let _shared_object = built.shared_object();

            // Without an fcinfo, we can't call this.
            let _loaded = unsafe { built.load()? };

            Ok(())
        }
        wrapped().unwrap()
    }
}
