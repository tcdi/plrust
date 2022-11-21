use crate::{
    plrust_lang_oid,
    user_crate::{parse_source_and_deps, CrateState, CrateVariant, StateProvisioned},
    PlRustError,
};
use eyre::WrapErr;
use pgx::{pg_sys, FromDatum, IntoDatum, PgBox, PgOid};
use quote::quote;
use std::path::Path;

impl CrateState for StateGenerated {}

#[must_use]
pub(crate) struct StateGenerated {
    db_oid: pg_sys::Oid,
    fn_oid: pg_sys::Oid,
    user_dependencies: toml::value::Table,
    user_code: syn::Block,
    variant: CrateVariant,
}

impl StateGenerated {
    #[cfg(any(test, feature = "pg_test"))]
    pub(crate) fn for_tests(
        db_oid: pg_sys::Oid,
        fn_oid: pg_sys::Oid,
        user_deps: toml::value::Table,
        user_code: syn::Block,
        variant: CrateVariant,
    ) -> Self {
        Self {
            db_oid,
            fn_oid,
            user_dependencies: user_deps.into(),
            user_code,
            variant,
        }
    }

    #[tracing::instrument(level = "debug")]
    #[allow(unsafe_op_in_unsafe_fn)] // TODO: A bit of a mess in here, fix this?
    pub(crate) unsafe fn try_from_fn_oid(
        db_oid: pg_sys::Oid,
        fn_oid: pg_sys::Oid,
    ) -> eyre::Result<Self> {
        let proc_tuple = pg_sys::SearchSysCache(
            pg_sys::SysCacheIdentifier_PROCOID as i32,
            fn_oid.into_datum().unwrap(), // TODO: try_from_datum
            pg_sys::Datum::from(0_u64),
            pg_sys::Datum::from(0_u64),
            pg_sys::Datum::from(0_u64),
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
        let lang_oid = pg_sys::Oid::from_datum(lang_datum, is_null);
        if lang_oid != plrust_lang_oid() {
            return Err(PlRustError::NotPlRustFunction(fn_oid))?;
        }

        let prosrc_datum = pg_sys::SysCacheGetAttr(
            pg_sys::SysCacheIdentifier_PROCOID as i32,
            proc_tuple,
            pg_sys::Anum_pg_proc_prosrc as pg_sys::AttrNumber,
            &mut is_null,
        );
        let (user_code, user_dependencies) = parse_source_and_deps(
            &String::from_datum(prosrc_datum, is_null).ok_or(PlRustError::NullSourceCode)?,
        )?;

        let proc_entry = PgBox::from_pg(pg_sys::heap_tuple_get_struct::<pg_sys::FormData_pg_proc>(
            proc_tuple,
        ));

        let return_oid = PgOid::from(proc_entry.prorettype);

        let variant = match return_oid == pgx::PgBuiltInOids::TRIGGEROID.oid() {
            true => CrateVariant::trigger(),
            false => {
                let argnames_datum = pg_sys::SysCacheGetAttr(
                    pg_sys::SysCacheIdentifier_PROCOID as i32,
                    proc_tuple,
                    pg_sys::Anum_pg_proc_proargnames as pg_sys::AttrNumber,
                    &mut is_null,
                );
                let argnames = Vec::<Option<_>>::from_datum(argnames_datum, is_null);

                let argtypes_datum = pg_sys::SysCacheGetAttr(
                    pg_sys::SysCacheIdentifier_PROCOID as i32,
                    proc_tuple,
                    pg_sys::Anum_pg_proc_proargtypes as pg_sys::AttrNumber,
                    &mut is_null,
                );
                let argtypes = Vec::<_>::from_datum(argtypes_datum, is_null).unwrap();

                let mut argument_oids_and_names = Vec::new();
                for i in 0..proc_entry.pronargs as usize {
                    let type_oid = argtypes.get(i).expect("no type_oid for argument");
                    let name = argnames.as_ref().and_then(|v| v.get(i).cloned()).flatten();

                    argument_oids_and_names.push((PgOid::from(*type_oid), name));
                }

                let is_strict = proc_entry.proisstrict;
                let return_set = proc_entry.proretset;

                CrateVariant::function(argument_oids_and_names, return_oid, return_set, is_strict)?
            }
        };

        pg_sys::ReleaseSysCache(proc_tuple);

        Ok(Self {
            db_oid,
            fn_oid,
            user_code,
            user_dependencies,
            variant,
        })
    }
    pub(crate) fn crate_name(&self) -> String {
        crate::plrust::crate_name(self.db_oid, self.fn_oid)
    }

    #[tracing::instrument(level = "debug", skip_all, fields(db_oid = %self.db_oid, fn_oid = %self.fn_oid))]
    pub(crate) fn lib_rs(&self) -> eyre::Result<syn::File> {
        let mut skeleton: syn::File = syn::parse_quote!(
            #![deny(unsafe_op_in_unsafe_fn)]
            use pgx::prelude::*;
        );

        let crate_name = self.crate_name();
        #[cfg(any(
            all(target_os = "macos", target_arch = "x86_64"),
            feature = "force_enable_x86_64_darwin_generations"
        ))]
        let crate_name = {
            let mut crate_name = crate_name;
            let next = crate::generation::next_generation(&crate_name, true).unwrap_or_default();

            crate_name.push_str(&format!("_{}", next));
            crate_name
        };
        let symbol_ident = proc_macro2::Ident::new(&crate_name, proc_macro2::Span::call_site());

        tracing::trace!(symbol_name = %crate_name, "Generating `lib.rs`");

        let user_code = &self.user_code;
        let user_function = match &self.variant {
            CrateVariant::Function {
                ref arguments,
                ref return_type,
                ..
            } => {
                let user_fn: syn::ItemFn = syn::parse2(quote! {
                    #[pg_extern]
                    fn #symbol_ident(
                        #( #arguments ),*
                    ) -> #return_type
                    #user_code
                })
                .wrap_err("Parsing generated user function")?;
                user_fn
            }
            CrateVariant::Trigger => {
                let user_fn: syn::ItemFn = syn::parse2(quote! {
                    #[pg_trigger]
                    fn #symbol_ident(
                        trigger: &::pgx::PgTrigger,
                    ) -> core::result::Result<
                        ::pgx::heap_tuple::PgHeapTuple<'_, impl ::pgx::WhoAllocated<::pgx::pg_sys::HeapTupleData>>,
                        Box<dyn std::error::Error>,
                    > #user_code
                })
                .wrap_err("Parsing generated user trigger")?;
                user_fn
            }
        };

        skeleton.items.push(user_function.into());
        Ok(skeleton)
    }

    #[tracing::instrument(level = "debug", skip_all, fields(db_oid = %self.db_oid, fn_oid = %self.fn_oid))]
    pub(crate) fn cargo_toml(&self) -> eyre::Result<toml::value::Table> {
        let major_version = pgx::pg_sys::get_pg_major_version_num();
        let version_feature = format!("pgx/pg{major_version}");
        let crate_name = self.crate_name();

        #[cfg(any(
            all(target_os = "macos", target_arch = "x86_64"),
            feature = "force_enable_x86_64_darwin_generations"
        ))]
        let crate_name = {
            let mut crate_name = crate_name;
            let next = crate::generation::next_generation(&crate_name, true).unwrap_or_default();

            crate_name.push_str(&format!("_{}", next));
            crate_name
        };

        tracing::trace!(
            crate_name = %crate_name,
            user_dependencies = ?self.user_dependencies.keys().cloned().collect::<Vec<String>>(),
            "Generating `Cargo.toml`"
        );

        let cargo_toml = toml::toml! {
                    [package]
                    edition = "2021"
                    name = crate_name
                    version = "0.0.0"

                    [features]
                    default = [version_feature]

                    [lib]
                    crate-type = ["cdylib"]

                    [dependencies]
                    pgx =  { version = "0.6.0-alpha.1", features = ["plrust"] }
                    pallocator = { version = "0.1.0", git = "https://github.com/tcdi/postgrestd", branch = "1.61" }
                    /* User deps added here */

                    [profile.release]
                    debug-assertions = true
                    codegen-units = 1_usize
                    lto = "fat"
                    opt-level = 3_usize
                    panic = "unwind"
        };

        match cargo_toml {
            toml::Value::Table(mut cargo_manifest) => {
                // We have to add the user deps now before we return it.
                match cargo_manifest.entry("dependencies") {
                    toml::value::Entry::Occupied(ref mut occupied) => match occupied.get_mut() {
                        toml::Value::Table(dependencies) => {
                            for (user_dep_name, user_dep_version) in &self.user_dependencies {
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
                                pgx_table
                                    .insert("path".into(), toml::Value::String(path.to_string()));
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
    #[tracing::instrument(level = "debug", skip_all, fields(db_oid = %self.db_oid, fn_oid = %self.fn_oid, parent_dir = %parent_dir.display()))]
    pub(crate) fn provision(&self, parent_dir: &Path) -> eyre::Result<StateProvisioned> {
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

        Ok(StateProvisioned::new(
            self.db_oid,
            self.fn_oid,
            crate_name,
            crate_dir,
        ))
    }

    pub(crate) fn fn_oid(&self) -> &u32 {
        &self.fn_oid
    }

    pub(crate) fn db_oid(&self) -> &u32 {
        &self.db_oid
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgx::pg_schema]
mod tests {
    use super::*;
    use pgx::*;
    use syn::parse_quote;

    #[pg_test]
    fn strict_string() {
        fn wrapped() -> eyre::Result<()> {
            let fn_oid = 0 as pg_sys::Oid;
            let db_oid = 1 as pg_sys::Oid;

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
                StateGenerated::for_tests(db_oid, fn_oid, user_deps, user_code, variant);

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
            let fixture_lib_rs = parse_quote! {
                #![deny(unsafe_op_in_unsafe_fn)]
                use pgx::prelude::*;
                #[pg_extern]
                fn #symbol_ident(arg0: &str) -> Option<String> {
                    Some(arg0.to_string())
                }
            };
            assert_eq!(
                prettyplease::unparse(&generated_lib_rs),
                prettyplease::unparse(&fixture_lib_rs),
                "Generated `lib.rs` differs from test (after formatting)",
            );
            Ok(())
        }
        wrapped().unwrap()
    }

    #[pg_test]
    fn non_strict_integer() {
        fn wrapped() -> eyre::Result<()> {
            let fn_oid = 0 as pg_sys::Oid;
            let db_oid = 1 as pg_sys::Oid;

            let variant = {
                let argument_oids_and_names = vec![(
                    PgOid::from(PgBuiltInOids::INT4OID.value()),
                    Some("val".into()),
                )];
                let return_oid = PgOid::from(PgBuiltInOids::INT8OID.value());
                let is_strict = false;
                let return_set = false;
                CrateVariant::function(argument_oids_and_names, return_oid, return_set, is_strict)?
            };
            let user_deps = toml::value::Table::default();
            let user_code = syn::parse2(quote! {
                { val.map(|v| v as i64) }
            })?;

            let generated =
                StateGenerated::for_tests(db_oid, fn_oid, user_deps, user_code, variant);

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
            let fixture_lib_rs = parse_quote! {
                #![deny(unsafe_op_in_unsafe_fn)]
                use pgx::prelude::*;
                #[pg_extern]
                fn #symbol_ident(val: Option<i32>) -> Option<i64> {
                    val.map(|v| v as i64)
                }
            };
            assert_eq!(
                prettyplease::unparse(&generated_lib_rs),
                prettyplease::unparse(&fixture_lib_rs),
                "Generated `lib.rs` differs from test (after formatting)",
            );
            Ok(())
        }
        wrapped().unwrap()
    }

    #[pg_test]
    fn strict_string_set() {
        fn wrapped() -> eyre::Result<()> {
            let fn_oid = 0 as pg_sys::Oid;
            let db_oid = 1 as pg_sys::Oid;

            let variant = {
                let argument_oids_and_names = vec![(
                    PgOid::from(PgBuiltInOids::TEXTOID.value()),
                    Some("val".into()),
                )];
                let return_oid = PgOid::from(PgBuiltInOids::TEXTOID.value());
                let is_strict = true;
                let return_set = true;
                CrateVariant::function(argument_oids_and_names, return_oid, return_set, is_strict)?
            };
            let user_deps = toml::value::Table::default();
            let user_code = syn::parse2(quote! {
                { Some(std::iter::repeat(val).take(5)) }
            })?;

            let generated =
                StateGenerated::for_tests(db_oid, fn_oid, user_deps, user_code, variant);

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
            let fixture_lib_rs = parse_quote! {
                #![deny(unsafe_op_in_unsafe_fn)]
                use pgx::prelude::*;
                #[pg_extern]
                fn #symbol_ident(val: &str) -> Option<::pgx::iter::SetOfIterator<Option<String>>> {
                    Some(std::iter::repeat(val).take(5))
                }
            };
            assert_eq!(
                prettyplease::unparse(&generated_lib_rs),
                prettyplease::unparse(&fixture_lib_rs),
                "Generated `lib.rs` differs from test (after formatting)",
            );
            Ok(())
        }
        wrapped().unwrap()
    }

    #[pg_test]
    fn trigger() {
        fn wrapped() -> eyre::Result<()> {
            let fn_oid = 0 as pg_sys::Oid;
            let db_oid = 1 as pg_sys::Oid;

            let variant = CrateVariant::trigger();
            let user_deps = toml::value::Table::default();
            let user_code = syn::parse2(quote! {
                { Ok(trigger.current().unwrap().into_owned()) }
            })?;

            let generated =
                StateGenerated::for_tests(db_oid, fn_oid, user_deps, user_code, variant);

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
            let fixture_lib_rs = parse_quote! {
                #![deny(unsafe_op_in_unsafe_fn)]
                use pgx::prelude::*;
                #[pg_trigger]
                fn #symbol_ident(
                    trigger: &::pgx::PgTrigger,
                ) -> core::result::Result<
                    ::pgx::heap_tuple::PgHeapTuple<'_, impl ::pgx::WhoAllocated<::pgx::pg_sys::HeapTupleData>>,
                    Box<dyn std::error::Error>,
                > {
                    Ok(trigger.current().unwrap().into_owned())
                }
            };
            assert_eq!(
                prettyplease::unparse(&generated_lib_rs),
                prettyplease::unparse(&fixture_lib_rs),
                "Generated `lib.rs` differs from test (after formatting)",
            );
            Ok(())
        }
        wrapped().unwrap()
    }
}
