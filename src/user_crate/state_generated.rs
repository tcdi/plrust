use crate::{
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
    fn_oid: pg_sys::Oid,
    user_dependencies: toml::value::Table,
    user_code: syn::Block,
    variant: CrateVariant,
}

impl StateGenerated {
    #[cfg(any(test, feature = "pg_test"))]
    pub(crate) fn for_tests(
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

    #[tracing::instrument(level = "debug")]
    pub(crate) unsafe fn try_from_fn_oid(fn_oid: pg_sys::Oid) -> eyre::Result<Self> {
        let proc_tuple = pg_sys::SearchSysCache(
            pg_sys::SysCacheIdentifier_PROCOID as i32,
            fn_oid.into_datum().unwrap(), // TODO: try_from_datum
            0.into(),
            0.into(),
            0.into(),
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
            &String::from_datum(prosrc_datum, is_null).ok_or(PlRustError::NullSourceCode)?,
        )?;
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

        let proc_entry = PgBox::from_pg(pg_sys::heap_tuple_get_struct::<pg_sys::FormData_pg_proc>(
            proc_tuple,
        ));

        let mut argument_oids_and_names = Vec::new();
        for i in 0..proc_entry.pronargs as usize {
            let type_oid = argtypes.get(i).expect("no type_oid for argument");
            let name = argnames.as_ref().and_then(|v| v.get(i).cloned()).flatten();

            argument_oids_and_names.push((PgOid::from(*type_oid), name));
        }

        let is_strict = proc_entry.proisstrict;
        let (return_oid, return_set) = (PgOid::from(proc_entry.prorettype), proc_entry.proretset);

        pg_sys::ReleaseSysCache(proc_tuple);

        let variant =
            CrateVariant::function(argument_oids_and_names, return_oid, return_set, is_strict)?;
        Ok(Self {
            fn_oid,
            user_code,
            user_dependencies,
            variant,
        })
    }
    pub(crate) fn crate_name(&self) -> String {
        crate::plrust::crate_name(self.fn_oid)
    }

    #[tracing::instrument(level = "debug", skip_all, fields(fn_oid = %self.fn_oid))]
    pub(crate) fn lib_rs(&self) -> eyre::Result<syn::File> {
        let mut skeleton: syn::File =
            syn::parse_str(include_str!("./skeleton.rs")).wrap_err("Parsing skeleton code")?;
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

        let user_function = match &self.variant {
            CrateVariant::Function {
                ref arguments,
                ref return_type,
                ..
            } => {
                let arguments = arguments.values();
                let user_code = &self.user_code;
                let file: syn::ItemFn = syn::parse2(quote! {
                    #[pg_extern]
                    fn #symbol_ident(
                        #( #arguments ),*
                    ) -> #return_type
                    #user_code
                })
                .wrap_err("Parsing generated user function")?;
                file
            }
        };
        skeleton.items.push(syn::Item::Fn(user_function));
        Ok(skeleton)
    }

    #[tracing::instrument(level = "debug", skip_all, fields(fn_oid = %self.fn_oid))]
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
            name = crate_name
            version = "0.0.0"
            edition = "2021"

            [lib]
            crate-type = ["cdylib"]

            [features]
            default = [ version_feature ]

            [dependencies]
            pgx = { version = "0.5.0-beta.0", features = ["postgrestd"], git = "https://github.com/tcdi/pgx", branch = "develop" }
            /* User deps added here */

            [profile.release]
            debug-assertions = true
            panic = "unwind"
            opt-level = 3_usize
            lto = "fat"
            codegen-units = 1_usize

            [patch.crates-io]
            libc = { git = "https://github.com/workingjubilee/libc", branch = "postgres-os" }
            getrandom = { git = "https://github.com/workingjubilee/getrandom", branch = "postgres-os" }
            ring = { git = "https://github.com/workingjubilee/ring", branch = "postgres-os" }
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
    #[tracing::instrument(level = "debug", skip_all, fields(fn_oid = %self.fn_oid, parent_dir = %parent_dir.display()))]
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

        Ok(StateProvisioned::new(self.fn_oid, crate_name, crate_dir))
    }

    pub(crate) fn fn_oid(&self) -> &u32 {
        &self.fn_oid
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

            let generated = StateGenerated::for_tests(fn_oid, user_deps, user_code, variant);

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
                #![no_std]
                extern crate alloc;
                use ::core::alloc::{GlobalAlloc, Layout};
                #[allow(dead_code, unused_imports)]
                use ::alloc::{
                    string::{String, ToString},
                    vec, vec::Vec, boxed::Box,
                };
                use ::pgx::{*, pg_sys};
                struct PostAlloc;
                #[global_allocator]
                static PALLOC: PostAlloc = PostAlloc;
                unsafe impl ::core::alloc::GlobalAlloc for PostAlloc {
                    unsafe fn alloc(&self, layout: ::core::alloc::Layout) -> *mut u8 {
                        ::pgx::pg_sys::palloc(layout.size()).cast()
                    }
                    unsafe fn dealloc(&self, ptr: *mut u8, _layout: ::core::alloc::Layout) {
                        ::pgx::pg_sys::pfree(ptr.cast());
                    }
                    unsafe fn realloc(&self, ptr: *mut u8, _layout: Layout, new_size: usize) -> *mut u8 {
                        ::pgx::pg_sys::repalloc(ptr.cast(), new_size).cast()
                    }
                }
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
            Ok(())
        }
        wrapped().unwrap()
    }

    #[pg_test]
    fn non_strict_integer() {
        fn wrapped() -> eyre::Result<()> {
            let fn_oid = 0 as pg_sys::Oid;

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

            let generated = StateGenerated::for_tests(fn_oid, user_deps, user_code, variant);

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
                #![no_std]
                extern crate alloc;
                use ::core::alloc::{GlobalAlloc, Layout};
                #[allow(dead_code, unused_imports)]
                use ::alloc::{
                    string::{String, ToString},
                    vec, vec::Vec, boxed::Box,
                };
                use ::pgx::{*, pg_sys};
                struct PostAlloc;
                #[global_allocator]
                static PALLOC: PostAlloc = PostAlloc;
                unsafe impl ::core::alloc::GlobalAlloc for PostAlloc {
                    unsafe fn alloc(&self, layout: ::core::alloc::Layout) -> *mut u8 {
                        ::pgx::pg_sys::palloc(layout.size()).cast()
                    }
                    unsafe fn dealloc(&self, ptr: *mut u8, _layout: ::core::alloc::Layout) {
                        ::pgx::pg_sys::pfree(ptr.cast());
                    }
                    unsafe fn realloc(&self, ptr: *mut u8, _layout: Layout, new_size: usize) -> *mut u8 {
                        ::pgx::pg_sys::repalloc(ptr.cast(), new_size).cast()
                    }
                }
                #[pg_extern]
                fn #symbol_ident(val: Option<i32>) -> Option<i64> {
                    val.map(|v| v as i64)
                }
            };
            assert_eq!(
                generated_lib_rs,
                fixture_lib_rs,
                "Generated `lib.rs` differs from test (output formatted)\n\nGenerated:\n{}\nFixture:\n{}\n",
                prettyplease::unparse(&generated_lib_rs),
                prettyplease::unparse(&fixture_lib_rs)
            );
            Ok(())
        }
        wrapped().unwrap()
    }

    #[pg_test]
    fn strict_string_set() {
        fn wrapped() -> eyre::Result<()> {
            let fn_oid = 0 as pg_sys::Oid;

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

            let generated = StateGenerated::for_tests(fn_oid, user_deps, user_code, variant);

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
                #![no_std]
                extern crate alloc;
                use ::core::alloc::{GlobalAlloc, Layout};
                #[allow(dead_code, unused_imports)]
                use ::alloc::{
                    string::{String, ToString},
                    vec, vec::Vec, boxed::Box,
                };
                use ::pgx::{*, pg_sys};
                struct PostAlloc;
                #[global_allocator]
                static PALLOC: PostAlloc = PostAlloc;
                unsafe impl ::core::alloc::GlobalAlloc for PostAlloc {
                    unsafe fn alloc(&self, layout: ::core::alloc::Layout) -> *mut u8 {
                        ::pgx::pg_sys::palloc(layout.size()).cast()
                    }
                    unsafe fn dealloc(&self, ptr: *mut u8, _layout: ::core::alloc::Layout) {
                        ::pgx::pg_sys::pfree(ptr.cast());
                    }
                    unsafe fn realloc(&self, ptr: *mut u8, _layout: Layout, new_size: usize) -> *mut u8 {
                        ::pgx::pg_sys::repalloc(ptr.cast(), new_size).cast()
                    }
                }
                #[pg_extern]
                fn #symbol_ident(val: &str) -> Option<impl Iterator<Item = Option<String>> + '_> {
                    Some(std::iter::repeat(val).take(5))
                }
            };
            assert_eq!(
                generated_lib_rs,
                fixture_lib_rs,
                "Generated `lib.rs` differs from test (output formatted)\n\nGenerated:\n{}\nFixture:\n{}\n",
                prettyplease::unparse(&generated_lib_rs),
                prettyplease::unparse(&fixture_lib_rs)
            );
            Ok(())
        }
        wrapped().unwrap()
    }
}
