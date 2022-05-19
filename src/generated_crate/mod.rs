use pgx::{pg_sys, PgOid, PgBuiltInOids};
use std::{collections::HashMap, path::PathBuf, process::Command};
use crate::PlRustError;
use quote::quote;
use proc_macro2::{TokenStream, Ident, Span};
use eyre::{WrapErr, eyre};
use color_eyre::{Section, SectionExt};

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
    #[cfg(any(test, feature = "pg_test"))]
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
    crate_name: String,
}

impl CratePhase for CrateProvisioned {}

#[must_use]
pub struct CrateBuilt {
    shared_object: PathBuf,
}

impl CratePhase for CrateBuilt {}

impl GeneratableCrate<CrateGenerated> {
    #[cfg(any(test, feature = "pg_test"))]
    #[tracing::instrument(level = "debug", skip_all)]
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
    #[tracing::instrument(level = "debug", skip_all)]
    pub unsafe fn try_from_fn_oid(fn_oid: pg_sys::Oid) -> eyre::Result<Self> {
        todo!()
    }
    pub fn crate_name(&self) -> String {
        format!("plrust_fn_oid_{}", self.0.fn_oid)
    }
    #[tracing::instrument(level = "debug", skip_all)]
    pub fn lib_rs(&self) -> eyre::Result<syn::File> {
        match &self.0.variant {
            CrateVariant::Function {
                ref arguments,
                ref return_type,
                ..
            } => {
                let fn_ident = Ident::new(&self.crate_name(), Span::call_site());
                let arguments = arguments.values();
                let user_code = &self.0.user_code;
                let file: syn::File = syn::parse2(quote! {
                    use pgx::*;
        
                    #[pg_extern]
                    fn #fn_ident(
                        #( #arguments ),*
                    ) -> #return_type
                    #user_code
                }).wrap_err("Parsing generated user function")?;
                Ok(file)
            },
        }
    }
    #[tracing::instrument(level = "debug", skip_all)]
    pub fn cargo_toml(&self) -> eyre::Result<toml::value::Table> {
        let mut cargo_toml = toml::toml! {
            [package]
            /* Crate name here */
            version = "0.0.0"
            edition = "2021"

            [lib]
            crate-type = ["cdylib"]

            [features]
            default = [ /* PG major version feature here */ ]

            [dependencies]
            pgx = "0.4.3"
            /* User deps here */

            [profile.release]
            panic = "unwind"
            opt-level = 3_usize
            lto = "fat"
            codegen-units = 1_usize
        };

        match cargo_toml {
            toml::Value::Table(mut cargo_manifest) => {
                match cargo_manifest.entry("package") {
                    toml::value::Entry::Occupied(ref mut occupied) => match occupied.get_mut() {
                        toml::Value::Table(package) => match package.entry("name") {
                            entry @ toml::value::Entry::Vacant(_) => {
                                let _ = entry.or_insert(self.crate_name().into());
                            }
                            _ => {
                                return Err(PlRustError::GeneratingCargoToml)
                                    .wrap_err("Getting `[package]` field `name` as vacant")?
                            }
                        },
                        _ => {
                            return Err(PlRustError::GeneratingCargoToml)
                                .wrap_err("Getting `[package]` as table")?
                        }
                    },
                    _ => {
                        return Err(PlRustError::GeneratingCargoToml)
                            .wrap_err("Getting `[package]`")?
                    }
                };

                match cargo_manifest.entry("dependencies") {
                    toml::value::Entry::Occupied(ref mut occupied) => match occupied.get_mut() {
                        toml::Value::Table(dependencies) => {
                            for (user_dep_name, user_dep_version) in &self.0.user_deps {
                                dependencies.insert(user_dep_name.clone(), user_dep_version.clone());
                            }
                        }
                        _ => {
                            return Err(PlRustError::GeneratingCargoToml)
                                .wrap_err("Getting `[dependencies]` as table")?
                        }
                    },
                    _ => {
                        return Err(PlRustError::GeneratingCargoToml)
                            .wrap_err("Getting `[dependencies]`")?
                    }
                };

                match cargo_manifest.entry("features") {
                    toml::value::Entry::Occupied(ref mut occupied) => match occupied.get_mut() {
                        toml::Value::Table(dependencies) => match dependencies.entry("default") {
                            toml::value::Entry::Occupied(ref mut occupied) => {
                                match occupied.get_mut() {
                                    toml::Value::Array(default) => {
                                        let major_version = pgx::pg_sys::get_pg_major_version_num();
                                        default.push(format!("pgx/pg{major_version}").into())
                                    }
                                    _ => {
                                        return Err(PlRustError::GeneratingCargoToml).wrap_err(
                                            "Getting `[features]` field `default` as array",
                                        )?
                                    }
                                }
                            }
                            _ => {
                                return Err(PlRustError::GeneratingCargoToml)
                                    .wrap_err("Getting `[features]` field `default`")?
                            }
                        },
                        _ => {
                            return Err(PlRustError::GeneratingCargoToml)
                                .wrap_err("Getting `[features]` as table")?
                        }
                    },
                    _ => {
                        return Err(PlRustError::GeneratingCargoToml)
                            .wrap_err("Getting `[dependencies]`")?
                    }
                };

                Ok(cargo_manifest)
            }
            _ => {
                return Err(PlRustError::GeneratingCargoToml)
                    .wrap_err("Getting `Cargo.toml` as table")?
            }
        }
    }
    /// Provision into a given folder and return the crate directory.
    #[tracing::instrument(level = "debug", skip_all)]
    pub fn provision(
        &self,
        parent_dir: PathBuf,
    ) -> eyre::Result<GeneratableCrate<CrateProvisioned>> {
        let crate_name = self.crate_name();
        let crate_dir = parent_dir.join(&crate_name);
        let src_dir = crate_dir.join("src");
        std::fs::create_dir_all(&src_dir).wrap_err("Could not create crate directory in configured `plrust.work_dir` location")?;

        let lib_rs = self.lib_rs()?;
        let lib_rs_path = src_dir.join("lib.rs");
        std::fs::write(
            &lib_rs_path,
            &prettyplease::unparse(&lib_rs)
        ).wrap_err("Writing generated `lib.rs`")?;

        let cargo_toml = self.cargo_toml()?;
        let cargo_toml_path = crate_dir.join("Cargo.toml");
        std::fs::write(
            &cargo_toml_path,
            &toml::to_string(&cargo_toml).wrap_err("Stringifying generated `Cargo.toml`")?,
        ).wrap_err("Writing generated `Cargo.toml`")?;


        Ok(GeneratableCrate(CrateProvisioned { crate_name, crate_dir }))
    }
}

impl GeneratableCrate<CrateProvisioned> {
    #[tracing::instrument(level = "debug", skip_all)]
    pub fn build(self, target_dir: Option<PathBuf>, pg_config: PathBuf) -> eyre::Result<GeneratableCrate<CrateBuilt>> {
        let target_dir = target_dir.unwrap_or(self.0.crate_dir.join("target"));

        let cargo_output = Command::new("cargo")
            .current_dir(&self.0.crate_dir)
            .arg("build")
            .arg("--release")
            .env("PGX_PG_CONFIG_PATH", pg_config)
            .env("CARGO_TARGET_DIR", &target_dir)
            .env("CARGO_TARGET_DIR", &target_dir)
            .env(
                "RUSTFLAGS",
                "-Ctarget-cpu=native -Clink-args=-Wl,-undefined,dynamic_lookup",
            ).output().wrap_err("`cargo` execution failure")?;

        let stdout = String::from_utf8(cargo_output.stdout)
            .wrap_err("`cargo`'s stdout was not  UTF-8")?;
        let stderr = String::from_utf8(cargo_output.stderr)
            .wrap_err("`cargo`'s stderr was not  UTF-8")?;

        if cargo_output.status.success() {
            let crate_name = self.0.crate_name;
            use std::env::consts::DLL_SUFFIX;

            let shared_object = target_dir.join(&format!("lib{crate_name}{DLL_SUFFIX}"));
            Ok(GeneratableCrate(CrateBuilt { shared_object }))
        } else {
            Err(eyre!(PlRustError::CargoBuildFail)
                .section(stdout.header("`cargo build` stdout:"))
                .section(stderr.header("`cargo build` stderr:"))
                .with_section(|| 
                    std::fs::read_to_string(
                        &self.0.crate_dir.join("src").join("lib.rs")
                    ).wrap_err("Writing generated `lib.rs`").expect("Reading generated `lib.rs` to output during error")
                    .header("Source Code:")
                ))?
        }
    }
}

impl GeneratableCrate<CrateBuilt> {
    #[tracing::instrument(level = "debug", skip_all)]
    pub fn shared_object(&self) -> PathBuf {
        self.0.shared_object.clone()
    }
}

#[must_use]
pub enum CrateVariant {
    Function {
        arguments: HashMap<(PgOid, Option<String>), syn::FnArg>,
        return_oid: PgOid,
        return_type: syn::Type,
        return_set: bool,
        is_strict: bool,
    },
    // Trigger,
}

impl CrateVariant {
    #[tracing::instrument(level = "debug", skip_all)]
    fn function(
        argument_oids: Vec<(PgOid, Option<String>)>,
        return_oid: PgOid,
        return_set: bool,
        is_strict: bool,
    ) -> eyre::Result<Self> {
        let mut arguments = HashMap::default();
        for (idx, (argument_oid, maybe_argument_name)) in argument_oids.into_iter().enumerate() {
            let rust_type: syn::Type = {
                let bare = oid_to_syn_type(&argument_oid, false)?;
                match is_strict {
                    true => bare,
                    false => syn::parse2(quote! {
                        Option<#bare>
                    })
                    .wrap_err("Wrapping argument type")?,
                }
            };

            let argument_name = match &maybe_argument_name {
                Some(argument_name) => Ident::new(&argument_name.clone(), Span::call_site()),
                None => Ident::new(&format!("arg{}", idx), Span::call_site()),
            };
            let rust_pat_type: syn::FnArg = syn::parse2(quote! {
                #argument_name: #rust_type
            }).map_err(PlRustError::Parse).wrap_err("Making argument pattern type")?;
            arguments.insert((argument_oid, maybe_argument_name), rust_pat_type);
        }

        let return_type: syn::Type = {
            let bare = oid_to_syn_type(&return_oid, true)?;
            match return_set {
                true => syn::parse2(quote! { Option<impl Iterator<Item=Option<#bare>> + '_> })
                    .wrap_err("Wrapping return type")?,
                false => {
                    syn::parse2(quote! { Option<#bare> }).wrap_err("Wrapping return type")?
                }
            }
        };

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


#[cfg(any(test, feature = "pg_test"))]
#[pgx::pg_schema]
mod tests {
    use pgx::*;

    use super::*;
    use eyre::WrapErr;
    use quote::quote;
    use toml::toml;
    use syn::parse_quote;

    #[pg_test]
    fn function_workflow() {
        fn wrapped() -> eyre::Result<()> {
            let fn_oid = 0 as pg_sys::Oid;
            let target_dir = crate::gucs::work_dir();
            let pg_config = PathBuf::from(crate::gucs::pg_config());

            let variant = {
                let argument_oids_and_names = vec![
                    (PgOid::from(PgBuiltInOids::TEXTOID.value()), None)
                ];
                let return_oid = PgOid::from(PgBuiltInOids::TEXTOID.value());
                let is_strict = true;
                let return_set = false;
                CrateVariant::function(argument_oids_and_names, return_oid, return_set, is_strict)?
            };
            let user_deps = toml::value::Table::default();
            let user_code = syn::parse2(quote ! {
                { Some(arg0.to_string()) }
            })?;
    
            let generated = GeneratableCrate::generated_for_tests(
                fn_oid,
                user_deps,
                user_code,
                variant,
            );
    
            let generated_lib_rs = generated.lib_rs()?;
            let fixture_lib_rs = parse_quote! {
                use pgx::*;
                #[pg_extern]
                fn plrust_fn_oid_0(arg0: &str) -> Option<String> {
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
            let fixture_cargo_toml = toml! {
                [package]
                edition = "2021"
                name = "plrust_fn_oid_0"
                version = "0.0.0"

                [features]
                default = ["pgx/pg14"]

                [lib]
                crate-type = ["cdylib"]

                [dependencies]
                pgx = "0.4.3"

                [profile.release]
                codegen-units = 1_usize
                lto = "fat"
                opt-level = 3_usize
                panic = "unwind"
            };
            assert_eq!(
                generated_cargo_toml, 
                *fixture_cargo_toml.as_table().unwrap(),
                "Generated `Cargo.toml` differs from test (output formatted)\n\nGenerated:\n{}\nFixture:\n{}\n",
                toml::to_string(&generated_cargo_toml)?,
                toml::to_string(&fixture_cargo_toml)?,
            );
    
    
            let parent_dir =
                tempdir::TempDir::new("plrust-generated-crate-function-workflow").wrap_err("Creating temp dir")?;
            let provisioned = generated.provision(parent_dir.into_path())?;
    
            let built = provisioned.build(Some(target_dir), pg_config)?;
    
            let shared_object = built.shared_object();
    
            Ok(())
        }
        wrapped().unwrap()
    }
}
