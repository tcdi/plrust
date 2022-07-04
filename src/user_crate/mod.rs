mod crate_variant;
mod state_built;
mod state_generated;
mod state_loaded;
mod state_provisioned;
mod target;

use crate_variant::CrateVariant;
pub(crate) use state_built::StateBuilt;
pub(crate) use state_generated::StateGenerated;
pub(crate) use state_loaded::StateLoaded;
pub(crate) use state_provisioned::StateProvisioned;

use crate::PlRustError;
use pgx::{pg_sys, PgBuiltInOids, PgOid};
use proc_macro2::TokenStream;
use quote::quote;
use std::{
    path::{Path, PathBuf},
    process::Output,
};

pub(crate) struct UserCrate<P: CrateState>(P);

pub(crate) trait CrateState {}

impl UserCrate<StateGenerated> {
    #[cfg(any(test, feature = "pg_test"))]
    #[tracing::instrument(level = "debug", skip_all)]
    pub(crate) fn generated_for_tests(
        fn_oid: pg_sys::Oid,
        user_deps: toml::value::Table,
        user_code: syn::Block,
        variant: CrateVariant,
    ) -> Self {
        Self(StateGenerated::for_tests(
            fn_oid,
            user_deps.into(),
            user_code,
            variant,
        ))
    }
    #[tracing::instrument(level = "debug", skip_all)]
    pub unsafe fn try_from_fn_oid(fn_oid: pg_sys::Oid) -> eyre::Result<Self> {
        StateGenerated::try_from_fn_oid(fn_oid).map(Self)
    }
    #[tracing::instrument(level = "debug", skip_all)]
    pub fn lib_rs(&self) -> eyre::Result<syn::File> {
        self.0.lib_rs()
    }
    #[tracing::instrument(level = "debug", skip_all)]
    pub fn cargo_toml(&self) -> eyre::Result<toml::value::Table> {
        self.0.cargo_toml()
    }
    /// Provision into a given folder and return the crate directory.
    #[tracing::instrument(level = "debug", skip_all, fields(fn_oid = %self.0.fn_oid()))]
    pub fn provision(&self, parent_dir: &Path) -> eyre::Result<UserCrate<StateProvisioned>> {
        self.0.provision(parent_dir).map(UserCrate)
    }
}

impl UserCrate<StateProvisioned> {
    #[tracing::instrument(
        level = "debug",
        skip_all,
        fields(
            fn_oid = %self.0.fn_oid(),
            crate_dir = %self.0.crate_dir().display(),
            target_dir = target_dir.map(|v| tracing::field::display(v.display())),
        ))]
    pub fn build(
        self,
        artifact_dir: &Path,
        pg_config: PathBuf,
        target_dir: Option<&Path>,
    ) -> eyre::Result<(UserCrate<StateBuilt>, Output)> {
        self.0
            .build(artifact_dir, pg_config, target_dir)
            .map(|(state, output)| (UserCrate(state), output))
    }
}

impl UserCrate<StateBuilt> {
    #[tracing::instrument(level = "debug")]
    pub(crate) fn built(fn_oid: pg_sys::Oid, shared_object: PathBuf) -> Self {
        UserCrate(StateBuilt::new(fn_oid, shared_object))
    }
    #[tracing::instrument(level = "debug", skip_all, fields(fn_oid = %self.0.fn_oid()))]
    pub fn shared_object(&self) -> &Path {
        self.0.shared_object()
    }
    #[tracing::instrument(level = "debug", skip_all, fields(fn_oid = %self.0.fn_oid()))]
    pub unsafe fn load(self) -> eyre::Result<UserCrate<StateLoaded>> {
        self.0.load().map(UserCrate)
    }
}

impl UserCrate<StateLoaded> {
    #[tracing::instrument(level = "debug", skip_all, fields(fn_oid = %self.fn_oid()))]
    pub unsafe fn evaluate(&self, fcinfo: pg_sys::FunctionCallInfo) -> pg_sys::Datum {
        self.0.evaluate(fcinfo)
    }

    pub(crate) fn close(self) -> eyre::Result<()> {
        self.0.close()
    }

    pub(crate) fn symbol_name(&self) -> &str {
        self.0.symbol_name()
    }

    pub(crate) fn fn_oid(&self) -> &u32 {
        self.0.fn_oid()
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
            PgBuiltInOids::NUMERICOID => quote! { Numeric },
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
    let mut code_block = String::new();
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

    let user_dependencies: toml::value::Table =
        toml::from_str(&deps_block).map_err(PlRustError::ParsingDependenciesBlock)?;

    let user_code: syn::Block =
        syn::parse_str(&format!("{{ {code_block} }}")).map_err(PlRustError::ParsingCodeBlock)?;

    Ok((user_code, user_dependencies))
}

#[cfg(any(test, feature = "pg_test"))]
#[pgx::pg_schema]
mod tests {
    use pgx::*;

    use crate::user_crate::*;
    use eyre::WrapErr;
    use quote::quote;
    use syn::parse_quote;
    use toml::toml;

    #[pg_test]
    fn full_workflow() {
        fn wrapped() -> eyre::Result<()> {
            let fn_oid = 0 as pg_sys::Oid;
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

            let generated = UserCrate::generated_for_tests(fn_oid, user_deps, user_code, variant);
            let crate_name = crate::plrust::crate_name(fn_oid);
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
            let fixture_lib_rs = parse_quote! {
                use pgx::{pg_sys, *};

                #[pg_extern]
                fn #symbol_ident(arg0: &str) -> Option<String> {
                    Some(arg0.to_string())
                }
            };
            assert_eq!(
                generated_lib_rs,
                fixture_lib_rs,
                "Generated `lib.rs` differs from test (output formatted)\n\nGenerated:\n{}\nFixture:\n{}\n",
                prettyplease::unparse(&generated_lib_rs),
                prettyplease::unparse(&fixture_lib_rs)
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
                pgx = { version = "0.5.0-beta.0", features = ["postgrestd"], git = "https://github.com/tcdi/pgx", branch = "develop" }
                /* User deps added here */

                [profile.release]
                debug-assertions = true
                codegen-units = 1_usize
                lto = "fat"
                opt-level = 3_usize
                panic = "unwind"

                [patch.crates-io]
                pgx-tests = { version = "0.5.0-beta.0", git = "https://github.com/tcdi/pgx", branch = "develop" }
                libc = { git = "https://github.com/workingjubilee/libc", branch = "postgres-os" }
                getrandom = { git = "https://github.com/workingjubilee/getrandom", branch = "postgres-os" }
                ring = { git = "https://github.com/workingjubilee/ring", branch = "postgres-os" }
            };
            assert_eq!(
                generated_cargo_toml,
                *fixture_cargo_toml.as_table().unwrap(),
                "Generated `Cargo.toml` differs from test (output formatted)\n\nGenerated:\n{}\nFixture:\n{}\n",
                toml::to_string(&generated_cargo_toml)?,
                toml::to_string(&fixture_cargo_toml)?,
            );

            let parent_dir = tempdir::TempDir::new("plrust-generated-crate-function-workflow")
                .wrap_err("Creating temp dir")?;
            let provisioned = generated.provision(parent_dir.path())?;

            let (built, _output) =
                provisioned.build(parent_dir.path(), pg_config, Some(target_dir.as_path()))?;

            let _shared_object = built.shared_object();

            // Without an fcinfo, we can't call this.
            let _loaded = unsafe { built.load()? };

            Ok(())
        }
        wrapped().unwrap()
    }
}
