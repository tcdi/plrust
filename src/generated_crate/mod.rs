use crate::PlRustError;
use color_eyre::{Section, SectionExt};
use eyre::{eyre, WrapErr};
use pgx::{
    datum::{FromDatum, IntoDatum},
    pg_sys, PgBox, PgBuiltInOids, PgOid,
};
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::{Command, Output},
};

pub struct GeneratableCrate<P: CratePhase>(P);

pub trait CratePhase {}

#[must_use]
pub struct CrateGenerated {
    pub fn_oid: pg_sys::Oid,
    pub user_dependencies: toml::value::Table,
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
            user_dependencies: user_deps.into(),
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
    output: Output,
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
        let proc_tuple = pg_sys::SearchSysCache(
            pg_sys::SysCacheIdentifier_PROCOID as i32,
            fn_oid.into_datum().unwrap(), // TODO: try_from_datum
            0,
            0,
            0,
        );
        if proc_tuple.is_null() {
            return Err(PlRustError::NullProcTuple)?;
        }

        let mut is_null = false;

        let lang_datum = pg_sys::SysCacheGetAttr(
            pg_sys::SysCacheIdentifier_PROCOID as i32,
            proc_tuple,
            pg_sys::Anum_pg_proc_prolang as pg_sys::AttrNumber,
            &mut is_null,
        );
        let lang_oid = pg_sys::Oid::from_datum(lang_datum, is_null, pg_sys::OIDOID);
        let plrust =
            std::ffi::CString::new("plrust").expect("Expected `\"plrust\"` to be a valid CString");
        if lang_oid != Some(pg_sys::get_language_oid(plrust.as_ptr(), false)) {
            return Err(PlRustError::NotPlRustFunction(fn_oid))?;
        }

        let prosrc_datum = pg_sys::SysCacheGetAttr(
            pg_sys::SysCacheIdentifier_PROCOID as i32,
            proc_tuple,
            pg_sys::Anum_pg_proc_prosrc as pg_sys::AttrNumber,
            &mut is_null,
        );
        let (user_code, user_dependencies) = parse_source_and_deps(
            &String::from_datum(prosrc_datum, is_null, pg_sys::TEXTOID)
                .ok_or(PlRustError::NullSourceCode)?,
        )?;
        let argnames_datum = pg_sys::SysCacheGetAttr(
            pg_sys::SysCacheIdentifier_PROCOID as i32,
            proc_tuple,
            pg_sys::Anum_pg_proc_proargnames as pg_sys::AttrNumber,
            &mut is_null,
        );
        let argnames = Vec::<Option<_>>::from_datum(argnames_datum, is_null, pg_sys::TEXTARRAYOID);

        let argtypes_datum = pg_sys::SysCacheGetAttr(
            pg_sys::SysCacheIdentifier_PROCOID as i32,
            proc_tuple,
            pg_sys::Anum_pg_proc_proargtypes as pg_sys::AttrNumber,
            &mut is_null,
        );
        let argtypes = Vec::<_>::from_datum(argtypes_datum, is_null, pg_sys::OIDARRAYOID).unwrap();

        let proc_entry = PgBox::from_pg(pg_sys::heap_tuple_get_struct::<pg_sys::FormData_pg_proc>(
            proc_tuple,
        ));

        let mut arguement_oids_and_names = Vec::new();
        for i in 0..proc_entry.pronargs as usize {
            let type_oid = argtypes.get(i).expect("no type_oid for argument");
            let name = argnames.as_ref().and_then(|v| v.get(i).cloned()).flatten();

            arguement_oids_and_names.push((PgOid::from(*type_oid), name));
        }

        let is_strict = proc_entry.proisstrict;
        let (return_oid, return_set) = (PgOid::from(proc_entry.prorettype), proc_entry.proretset);

        pg_sys::ReleaseSysCache(proc_tuple);

        let variant =
            CrateVariant::function(arguement_oids_and_names, return_oid, return_set, is_strict)?;
        Ok(Self(CrateGenerated {
            fn_oid,
            user_code,
            user_dependencies,
            variant,
        }))
    }
    pub fn crate_name(&self) -> String {
        crate::plrust::crate_name(self.0.fn_oid)
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
                })
                .wrap_err("Parsing generated user function")?;
                Ok(file)
            }
        }
    }
    #[tracing::instrument(level = "debug", skip_all)]
    pub fn cargo_toml(&self) -> eyre::Result<toml::value::Table> {
        let cargo_toml = toml::toml! {
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
                            for (user_dep_name, user_dep_version) in &self.0.user_dependencies {
                                dependencies
                                    .insert(user_dep_name.clone(), user_dep_version.clone());
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

                match std::env::var("PLRUST_EXPERIMENTAL_CRATES") {
                    Err(_) => (),
                    Ok(path) => {
                        match cargo_manifest
                            .entry("patch")
                            .or_insert(toml::Value::Table(Default::default()))
                            .as_table_mut()
                            .unwrap() // infallible
                            .entry("crates-io")
                        {
                            entry @ toml::value::Entry::Vacant(_) => {
                                let mut pgx_table = toml::value::Table::new();
                                pgx_table.insert("path".into(), toml::Value::String(path.to_string()));
                                let mut crates_io_table = toml::value::Table::new();
                                crates_io_table.insert("pgx".into(), toml::Value::Table(pgx_table));
                                entry.or_insert(toml::Value::Table(crates_io_table));
                            }
                            _ => {
                                return Err(PlRustError::GeneratingCargoToml).wrap_err(
                                    "Setting `[patch]`, already existed (and wasn't expected to)",
                                )?
                            }
                        }
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
    pub fn provision(&self, parent_dir: &Path) -> eyre::Result<GeneratableCrate<CrateProvisioned>> {
        let crate_name = self.crate_name();
        let crate_dir = parent_dir.join(&crate_name);
        let src_dir = crate_dir.join("src");
        std::fs::create_dir_all(&src_dir).wrap_err(
            "Could not create crate directory in configured `plrust.work_dir` location",
        )?;

        let lib_rs = self.lib_rs()?;
        let lib_rs_path = src_dir.join("lib.rs");
        std::fs::write(&lib_rs_path, &prettyplease::unparse(&lib_rs))
            .wrap_err("Writing generated `lib.rs`")?;

        let cargo_toml = self.cargo_toml()?;
        let cargo_toml_path = crate_dir.join("Cargo.toml");
        std::fs::write(
            &cargo_toml_path,
            &toml::to_string(&cargo_toml).wrap_err("Stringifying generated `Cargo.toml`")?,
        )
        .wrap_err("Writing generated `Cargo.toml`")?;

        Ok(GeneratableCrate(CrateProvisioned {
            crate_name,
            crate_dir,
        }))
    }
}

impl GeneratableCrate<CrateProvisioned> {
    #[tracing::instrument(level = "debug", skip_all)]
    pub fn build(
        self,
        artifact_dir: &Path,
        pg_config: PathBuf,
        target_dir: Option<&Path>,
    ) -> eyre::Result<GeneratableCrate<CrateBuilt>> {
        let mut command = Command::new("cargo");

        command.current_dir(&self.0.crate_dir);
        command.arg("build");
        command.arg("--release");
        command.env("PGX_PG_CONFIG_PATH", pg_config);
        if let Some(target_dir) = target_dir {
            command.env("CARGO_TARGET_DIR", &target_dir);
        }
        command.env(
            "RUSTFLAGS",
            "-Ctarget-cpu=native -Clink-args=-Wl,-undefined,dynamic_lookup",
        );

        let output = command.output().wrap_err("`cargo` execution failure")?;

        if output.status.success() {
            let crate_name = &self.0.crate_name;
            use std::env::consts::DLL_SUFFIX;

            let built_shared_object_name = &format!("lib{crate_name}{DLL_SUFFIX}");
            let built_shared_object = target_dir
                .map(|d| d.join("release").join(&built_shared_object_name))
                .unwrap_or_else(|| self.0.crate_dir.join("target").join("release").join(built_shared_object_name));
            let shared_object_name = &format!("{crate_name}{DLL_SUFFIX}");
            let shared_object = artifact_dir.join(&shared_object_name);

            std::fs::rename(&built_shared_object, &shared_object).wrap_err_with(|| eyre!(
                "renaming shared object from `{}` to `{}`",
                built_shared_object.display(),
                shared_object.display()
            ))?;

            Ok(GeneratableCrate(CrateBuilt {
                shared_object,
                output,
            }))
        } else {
            let stdout =
                String::from_utf8(output.stdout).wrap_err("`cargo`'s stdout was not  UTF-8")?;
            let stderr =
                String::from_utf8(output.stderr).wrap_err("`cargo`'s stderr was not  UTF-8")?;

            Err(eyre!(PlRustError::CargoBuildFail)
                .section(stdout.header("`cargo build` stdout:"))
                .section(stderr.header("`cargo build` stderr:"))
                .with_section(|| {
                    std::fs::read_to_string(&self.0.crate_dir.join("src").join("lib.rs"))
                        .wrap_err("Writing generated `lib.rs`")
                        .expect("Reading generated `lib.rs` to output during error")
                        .header("Source Code:")
                }))?
        }
    }
}

impl GeneratableCrate<CrateBuilt> {
    #[tracing::instrument(level = "debug", skip_all)]
    pub fn shared_object(&self) -> &Path {
        self.0.shared_object.as_path()
    }
    #[tracing::instrument(level = "debug", skip_all)]
    pub fn output(&self) -> &Output {
        &self.0.output
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
        arguement_oids_and_names: Vec<(PgOid, Option<String>)>,
        return_oid: PgOid,
        return_set: bool,
        is_strict: bool,
    ) -> eyre::Result<Self> {
        let mut arguments = HashMap::default();
        for (idx, (argument_oid, maybe_argument_name)) in
            arguement_oids_and_names.into_iter().enumerate()
        {
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
            })
            .map_err(PlRustError::Parse)
            .wrap_err("Making argument pattern type")?;
            arguments.insert((argument_oid, maybe_argument_name), rust_pat_type);
        }

        let return_type: syn::Type = {
            let bare = oid_to_syn_type(&return_oid, true)?;
            match return_set {
                true => syn::parse2(quote! { Option<impl Iterator<Item=Option<#bare>> + '_> })
                    .wrap_err("Wrapping return type")?,
                false => syn::parse2(quote! { Option<#bare> }).wrap_err("Wrapping return type")?,
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

    use super::*;
    use eyre::WrapErr;
    use quote::quote;
    use syn::parse_quote;
    use toml::toml;

    #[pg_test]
    fn function_workflow() {
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

            let generated =
                GeneratableCrate::generated_for_tests(fn_oid, user_deps, user_code, variant);

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

            let parent_dir = tempdir::TempDir::new("plrust-generated-crate-function-workflow")
                .wrap_err("Creating temp dir")?;
            let provisioned = generated.provision(parent_dir.path())?;

            let built =
                provisioned.build(parent_dir.path(), pg_config, Some(target_dir.as_path()))?;

            let _shared_object = built.shared_object();

            Ok(())
        }
        wrapped().unwrap()
    }
}
