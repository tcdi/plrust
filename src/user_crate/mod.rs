mod state_generated;
mod state_provisioned;
mod state_built;
mod crate_variant;
#[cfg(any(test, feature = "pg_test"))]
pub mod tests;

pub use state_generated::StateGenerated;
pub use state_provisioned::StateProvisioned;
pub use state_built::StateBuilt;
use crate_variant::CrateVariant;

use crate::PlRustError;
use pgx::{
    pg_sys, PgBuiltInOids, PgOid,
};
use proc_macro2::TokenStream;
use quote::quote;
use std::{
    path::{Path, PathBuf},
    process::Output,
};



pub struct UserCrate<P: CrateState>(P);

pub trait CrateState {}


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
    #[tracing::instrument(level = "debug", skip_all)]
    pub fn provision(&self, parent_dir: &Path) -> eyre::Result<UserCrate<StateProvisioned>> {
        self.0.provision(parent_dir).map(UserCrate)
    }
}

impl UserCrate<StateProvisioned> {
    #[tracing::instrument(level = "debug", skip_all)]
    pub fn build(
        self,
        artifact_dir: &Path,
        pg_config: PathBuf,
        target_dir: Option<&Path>,
    ) -> eyre::Result<UserCrate<StateBuilt>> {
        self.0.build(artifact_dir, pg_config, target_dir).map(UserCrate)
    }
}

impl UserCrate<StateBuilt> {
    #[tracing::instrument(level = "debug", skip_all)]
    pub fn shared_object(&self) -> &Path {
        self.0.shared_object()
    }
    #[tracing::instrument(level = "debug", skip_all)]
    pub fn output(&self) -> &Output {
        &self.0.output()
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
