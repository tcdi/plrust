use pgx::{pg_sys, PgOid};
use std::{collections::HashMap, path::PathBuf};
use crate::PlRustError;

pub struct GeneratableCrate<P: CratePhase>(P);

pub trait CratePhase {}

#[must_use]
pub struct CrateGenerated {
    pub fn_oid: pg_sys::Oid,
    pub user_deps: toml::value::Table,
    pub user_code: syn::Block,
    pub variant: CrateVariant,
}

impl CratePhase for CrateGenerated {}

impl CrateGenerated { 
    #[cfg(test)]
    fn for_tests(
        fn_oid: pg_sys::Oid,
        user_deps: toml::value::Table,
        user_code: syn::Block,
        variant: CrateVariant,
    ) -> Self {
        Self {
            fn_oid,
            user_deps: user_deps.into(),
            user_code,
            variant,
        }
    }
}

#[must_use]
pub struct CrateProvisioned {
    crate_dir: PathBuf,
}

impl CratePhase for CrateProvisioned {}

#[must_use]
pub struct CrateBuilt {
    shared_object: PathBuf,
}

impl CratePhase for CrateBuilt {}

impl GeneratableCrate<CrateGenerated> {
    #[cfg(test)]
    fn generated_for_tests(
        fn_oid: pg_sys::Oid,
        user_deps: toml::value::Table,
        user_code: syn::Block,
        variant: CrateVariant,
    ) -> Self {
        Self(CrateGenerated::for_tests(
            fn_oid,
            user_deps.into(),
            user_code,
            variant,
        ))
    }
    pub unsafe fn try_from_fn_oid(fn_oid: pg_sys::Oid) -> eyre::Result<Self> {
        todo!()
    }
    pub fn lib_rs(&self) -> syn::File {
        todo!()
    }
    pub fn cargo_toml(&self) -> toml::value::Table {
        todo!()
    }
    /// Provision into a given folder and return the crate directory.
    pub fn provision(
        &self,
        parent_dir: PathBuf,
    ) -> eyre::Result<GeneratableCrate<CrateProvisioned>> {
        todo!();
        Ok(GeneratableCrate(CrateProvisioned { crate_dir: todo!() }))
    }
}

impl GeneratableCrate<CrateProvisioned> {
    pub fn build(self) -> eyre::Result<GeneratableCrate<CrateBuilt>> {
        todo!()
    }
}

impl GeneratableCrate<CrateBuilt> {
    pub fn shared_object(&self) -> PathBuf {
        todo!()
    }
}

#[must_use]
pub enum CrateVariant {
    Function {
        arguments: HashMap<(PgOid, Option<String>), syn::PatType>,
        return_oid: PgOid,
        return_type: syn::Type,
        return_set: bool,
        is_strict: bool,
    },
    // Trigger,
}

impl CrateVariant {
    #[cfg(test)]
    fn function_for_tests(
        argument_oids: Vec<(PgOid, Option<String>)>,
        return_oid: PgOid,
        return_set: bool,
        is_strict: bool,
        is_set: bool,
    ) -> eyre::Result<Self> {
        let arguments = Default::default();
        for (argument_oid, argument_name) in argument_oids {
            let rust_type = oid_to_syn_type(&argument_oid, false);
            arguments.insert((argument_oid, argument_name), rust_type);
        }

        let return_type = make_rust_type(&return_oid, true)
            .ok_or(PlRustError::NoOidToRustMapping(return_oid))?;

        Ok(Self::Function {
            arguments,
            return_oid,
            return_type,
            return_set,
            is_strict,
        })
    }
}

#[tracing::instrument(level = "debug", skip_all, fields(type_oid = type_oid.value()))]
fn oid_to_syn_type(type_oid: &PgOid, owned: bool) -> Result<syn::Type, PlRustError> {
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
            PgBuiltInOids::BYTEAOID => quote! { &[u8] },
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
            PgBuiltInOids::TEXTOID => quote! { &str },
            PgBuiltInOids::TEXTOID if owned => quote! { String },
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


#[cfg(test)]
mod test {
    use super::*;
    use eyre::WrapErr;
    use quote::quote;

    #[test]
    fn workflow() -> eyre::Result<()> {
        let fn_oid = 0 as pg_sys::Oid;

        let variant = {
            let argument_oids_and_names = vec![(PgOid::Custom(0), None)];
            let return_oid = PgOid::Custom(0);
            let is_strict = true;
            CrateVariant::function_for_tests(argument_oids_and_names, return_oid, is_strict)?
        };
        let user_deps = toml::value::Table::default();
        let user_code = syn::parse2(quote ! {
            Some(1 + 1)
        })?;

        let generated = GeneratableCrate::generated_for_tests(
            fn_oid,
            user_deps,
            user_code,
            variant,
        );

        let parent_dir =
            tempdir::TempDir::new("plrust-generated-crate-workflow").wrap_err("Creating temp dir")?;
        let provisioned = generated.provision(parent_dir.into_path())?;

        let built = provisioned.build()?;

        let shared_object = built.shared_object();

        Ok(())
    }
}
