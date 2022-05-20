/*
Portions Copyright 2020-2021 ZomboDB, LLC.
Portions Copyright 2021-2022 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

use crate::{error::PlRustError, gucs};
use color_eyre::section::{Section, SectionExt};
use eyre::{eyre, Result, WrapErr};
use libloading::{Library, Symbol};
use once_cell::unsync::Lazy;
use pgx::{pg_sys::heap_tuple_get_struct, *};
use proc_macro2::TokenStream;
use quote::quote;
use std::{
    collections::{hash_map::Entry, HashMap},
    env::consts::DLL_SUFFIX,
    path::PathBuf,
    process::Command,
};

static mut LOADED_SYMBOLS: Lazy<
    HashMap<
        pg_sys::Oid,
        (
            Library,
            Option<
                Symbol<'static, unsafe extern "C" fn(pg_sys::FunctionCallInfo) -> pg_sys::Datum>,
            >,
        ),
    >,
> = Lazy::new(|| Default::default());

pub(crate) fn init() {
    ()
}

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
pub mod generation {
    /*!
        Darwin x86_64 is a peculiar platform for `dlclose`, this exists for a workaround to support
        `CREATE OR REPLACE FUNCTION`.

        If we unload something from `LOADED_SYMBOLS`, then load a recreated `so`, Darwin will have never
        properly unloaded it, and will load the old shared object (and the old symbol). This is surprising
        behavior to the user, and does not offer a good experience.

        Instead, we create a 'generation' for each build, and always load the largest numbered `so`. Since
        these `so`s are unique, Darwin loads the new one correctly. This technically 'leaks', but only
        because Darwin's `dlclose` 'leaks'.

        **This behavior is not required on other operating systems or architectures.**

        We expected this to also be required on Darwin aarch64, but testing on hardware has proven otherwise.

        See https://github.com/rust-lang/rust/issues/28794#issuecomment-368693049 which cites
        https://developer.apple.com/videos/play/wwdc2017/413/?time=1776.
    !*/

    use super::*;
    use std::fs;

    #[derive(thiserror::Error, Debug)]
    pub enum Error {
        #[error("No generations found (Mac OS x86_64 specific)")]
        NoGenerations,
        #[error("std::io::Error: {0}")]
        StdIoError(#[from] std::io::Error),
    }

    /// Find existing generations of a given prefix.
    #[tracing::instrument(level = "debug")]
    pub(crate) fn all_generations(
        prefix: &str,
    ) -> Result<Box<dyn Iterator<Item = (usize, PathBuf)> + '_>, Error> {
        let work_dir = gucs::work_dir();
        let filtered = fs::read_dir(work_dir)?
            .flat_map(|entry| {
                let path = entry.ok()?.path();
                let stem = path.file_stem().and_then(|f| f.to_str())?.to_string();
                Some((stem, path))
            })
            .filter(move |(stem, _path)| stem.starts_with(prefix))
            .flat_map(|(stem, path)| {
                let generation = stem.split('_').last()?;
                let generation = generation.parse::<usize>().ok()?;
                Some((generation, path))
            });

        Ok(Box::from(filtered))
    }

    /// Get the next generation number to be created.
    ///
    /// If `vacuum` is set, this will pass the setting on to [`latest_generation`].
    #[tracing::instrument(level = "debug")]
    pub(crate) fn next_generation(prefix: &str, vacuum: bool) -> Result<usize, Error> {
        let latest = latest_generation(prefix, vacuum);
        Ok(latest.map(|this| this.0 + 1).unwrap_or_default())
    }

    /// Get the latest created generation night.
    ///
    /// If `vacuum` is set, this garbage collect old `so` files.
    #[tracing::instrument(level = "debug")]
    pub(crate) fn latest_generation(prefix: &str, vacuum: bool) -> Result<(usize, PathBuf), Error> {
        let mut generations = all_generations(prefix)?.collect::<Vec<_>>();
        // We could use max_by, but might need to vacuum.
        generations.sort_by_key(|(generation, _path)| *generation);
        let latest = generations.pop();

        if vacuum {
            for (_index, old_path) in generations {
                pgx::info!("Vacuuming {:?}", old_path);
                std::fs::remove_file(old_path)?;
            }
        }

        latest.ok_or(Error::NoGenerations)
    }
}

#[tracing::instrument(level = "debug")]
pub(crate) unsafe fn unload_function(fn_oid: pg_sys::Oid) {
    let removed = LOADED_SYMBOLS.remove(&fn_oid);
    if let Some(_symbol) = removed {
        tracing::info!("unloaded function");
    }
}

#[tracing::instrument(level = "debug")]
pub(crate) unsafe fn lookup_function(
    fn_oid: pg_sys::Oid,
) -> Result<
    &'static Symbol<'static, unsafe extern "C" fn(pg_sys::FunctionCallInfo) -> pg_sys::Datum>,
    PlRustError,
> {
    let &mut (ref mut library, ref mut symbol) = match LOADED_SYMBOLS.entry(fn_oid) {
        entry @ Entry::Occupied(_) => {
            entry.or_insert_with(|| unreachable!("Occupied entry was vacant"))
        }
        entry @ Entry::Vacant(_) => {
            let crate_name = crate_name(fn_oid);

            #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
            let crate_name = {
                let mut crate_name = crate_name;
                let latest = generation::latest_generation(&crate_name, true)
                    .expect("Could not find latest generation.")
                    .0;

                crate_name.push_str(&format!("_{}", latest));
                crate_name
            };

            let shared_library = gucs::work_dir().join(&format!("{crate_name}{DLL_SUFFIX}"));
            let library = Library::new(&shared_library)?;

            entry.or_insert((library, None))
        }
    };

    match symbol {
        Some(symbol) => Ok(symbol),
        None => {
            let symbol_name = format!("plrust_fn_{}_wrapper", fn_oid);
            let inserted_symbol = symbol.insert(library.get(&symbol_name.as_bytes())?);

            Ok(inserted_symbol)
        }
    }
}

#[tracing::instrument(level = "debug")]
pub(crate) fn compile_function(fn_oid: pg_sys::Oid) -> eyre::Result<(PathBuf, String, String)> {
    let work_dir = gucs::work_dir();
    let (crate_name, crate_dir) = crate_name_and_path(fn_oid);

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    let crate_name = {
        let mut crate_name = crate_name;
        let latest = generation::next_generation(&crate_name, true)?;
        crate_name.push_str(&format!("_{}", latest));
        crate_name
    };

    // We need a `src` dir, so do it all at once
    let src = crate_dir.join("src");
    std::fs::create_dir_all(&src)
        .wrap_err("Could not create crate directory in configured `plrust.work_dir` location")?;

    let (user_code, user_dependencies, args, (return_type, is_set), is_strict) =
        extract_code_and_args(fn_oid)?;

    // the actual source code in src/lib.rs
    let source_code =
        generate_function_source(fn_oid, &user_code, &args, &return_type, is_set, is_strict)?;
    let lib_rs = src.join("lib.rs");
    std::fs::write(&lib_rs, &prettyplease::unparse(&source_code))
        .wrap_err("Writing generated `lib.rs`")?;

    let source_cargo_toml =
        generate_cargo_toml(fn_oid, &user_dependencies, &crate_dir, &crate_name)?;
    let cargo_toml = crate_dir.join("Cargo.toml");
    std::fs::write(
        &cargo_toml,
        &toml::to_string(&source_cargo_toml).wrap_err("Stringifying generated `Cargo.toml`")?,
    )
    .wrap_err("Writing generated `Cargo.toml`")?;

    let cargo_output = Command::new("cargo")
        .current_dir(&crate_dir)
        .arg("rustc")
        .arg("--release")
        .arg("--offline")
        .env("PGX_PG_CONFIG_PATH", gucs::pg_config())
        .env("CARGO_TARGET_DIR", &work_dir)
        .env(
            "RUSTFLAGS",
            "-Ctarget-cpu=native -Clink-args=-Wl,-undefined,dynamic_lookup",
        )
        .output()
        .wrap_err("`cargo` execution failure")?;

    let stdout =
        String::from_utf8(cargo_output.stdout).wrap_err("`cargo`'s stdout was not  UTF-8")?;
    let stderr =
        String::from_utf8(cargo_output.stderr).wrap_err("`cargo`'s stderr was not  UTF-8")?;

    let (final_path, stdout, stderr) = if !cargo_output.status.success() {
        return Err(eyre!(PlRustError::CargoBuildFail)
            .section(stdout.header("`cargo build` stdout:"))
            .section(stderr.header("`cargo build` stderr:"))
            .section(prettyplease::unparse(&source_code).header("Source Code:")))?;
    } else {
        match find_shared_library(&crate_name).0 {
            Some(shared_library) => {
                let final_path = work_dir.join(&format!("{crate_name}{DLL_SUFFIX}"));

                // move the shared_library into its final location, which is
                // at the root of the configured `work_dir`
                std::fs::rename(&shared_library, &final_path).wrap_err_with(|| {
                    format!(
                        "Moving shared library `{}` to final path `{}`",
                        shared_library.display(),
                        final_path.display(),
                    )
                })?;

                (final_path, stdout, stderr)
            }
            None => return Err(PlRustError::SharedObjectNotFound)?,
        }
    };

    // no matter what happened, remove our crate directory, ignoring any error that might generate
    std::fs::remove_dir_all(&crate_dir).ok();

    Ok((final_path, stdout, stderr))
}

#[tracing::instrument(level = "debug")]
fn generate_cargo_toml(
    fn_oid: pg_sys::Oid,
    user_deps: &toml::value::Table,
    crate_dir: &PathBuf,
    crate_name: &str,
) -> eyre::Result<toml::Value> {
    let major_version = pg_sys::get_pg_major_version_num();

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
        toml::Value::Table(ref mut cargo_manifest) => {
            match cargo_manifest.entry("package") {
                toml::value::Entry::Occupied(ref mut occupied) => match occupied.get_mut() {
                    toml::Value::Table(package) => match package.entry("name") {
                        entry @ toml::value::Entry::Vacant(_) => {
                            let _ = entry.or_insert(crate_name.into());
                        }
                        _ => {
                            return Err(PlRustError::GeneratingCargoToml)
                                .wrap_err("Getting `#[package]` field `name` as vacant")?
                        }
                    },
                    _ => {
                        return Err(PlRustError::GeneratingCargoToml)
                            .wrap_err("Getting `#[features]` as table")?
                    }
                },
                _ => {
                    return Err(PlRustError::GeneratingCargoToml)
                        .wrap_err("Getting `#[dependencies]`")?
                }
            };

            match cargo_manifest.entry("features") {
                toml::value::Entry::Occupied(ref mut occupied) => match occupied.get_mut() {
                    toml::Value::Table(dependencies) => match dependencies.entry("default") {
                        toml::value::Entry::Occupied(ref mut occupied) => {
                            match occupied.get_mut() {
                                toml::Value::Array(default) => {
                                    default.push(format!("pgx/pg{major_version}").into())
                                }
                                _ => {
                                    return Err(PlRustError::GeneratingCargoToml).wrap_err(
                                        "Getting `#[features]` field `default` as array",
                                    )?
                                }
                            }
                        }
                        _ => {
                            return Err(PlRustError::GeneratingCargoToml)
                                .wrap_err("Getting `#[features]` field `default`")?
                        }
                    },
                    _ => {
                        return Err(PlRustError::GeneratingCargoToml)
                            .wrap_err("Getting `#[features]` as table")?
                    }
                },
                _ => {
                    return Err(PlRustError::GeneratingCargoToml)
                        .wrap_err("Getting `#[dependencies]`")?
                }
            };

            match cargo_manifest.entry("dependencies") {
                toml::value::Entry::Occupied(ref mut occupied) => match occupied.get_mut() {
                    toml::Value::Table(dependencies) => {
                        for (user_dep_name, user_dep_version) in user_deps {
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

            match std::env::var("PLRUST_EXPERIMENTAL_CRATES") {
                Err(_) => (),
                Ok(path) => match cargo_manifest.entry("patch") {
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
                },
            };
        }
        _ => {
            return Err(PlRustError::GeneratingCargoToml)
                .wrap_err("Getting `Cargo.toml` as table")?
        }
    }

    Ok(cargo_toml)
}

#[tracing::instrument(level = "debug")]
fn crate_name(fn_oid: pg_sys::Oid) -> String {
    let db_oid = unsafe { pg_sys::MyDatabaseId };
    let ns_oid = unsafe { pg_sys::get_func_namespace(fn_oid) };
    format!("fn{}_{}_{}", db_oid, ns_oid, fn_oid)
}

#[tracing::instrument(level = "debug")]
fn crate_name_and_path(fn_oid: pg_sys::Oid) -> (String, PathBuf) {
    let crate_name = crate_name(fn_oid);
    let crate_dir = gucs::work_dir().join(&crate_name);

    (crate_name, crate_dir)
}

#[tracing::instrument(level = "debug")]
fn find_shared_library(crate_name: &str) -> (Option<PathBuf>, &str) {
    let target_dir = gucs::work_dir().join("release");
    let so = target_dir.join(&format!("lib{crate_name}{DLL_SUFFIX}"));

    if so.exists() {
        (Some(so), crate_name)
    } else {
        (None, crate_name)
    }
}

#[tracing::instrument(level = "debug", skip_all, fields(fn_oid = %fn_oid, args = ?args, return_type = ?return_type))]
fn generate_function_source(
    fn_oid: pg_sys::Oid,
    user_code: &syn::Block,
    args: &Vec<(PgOid, Option<String>)>,
    return_type: &PgOid,
    is_set: bool,
    is_strict: bool,
) -> eyre::Result<syn::File> {
    let mut file = syn::parse_file(include_str!("./postalloc.rs"))?;

    let user_fn_name = &format!("plrust_fn_{}", fn_oid);
    let user_fn_ident = syn::Ident::new(user_fn_name, proc_macro2::Span::call_site());

    let mut user_fn_arg_idents: Vec<syn::Ident> = Vec::default();
    let mut user_fn_arg_types: Vec<syn::Type> = Vec::default();
    for (arg_idx, (arg_type_oid, arg_name)) in args.iter().enumerate() {
        let arg_ty = oid_to_syn_type(arg_type_oid, false).wrap_err("Mapping argument type")?;
        let arg_ty_wrapped = match is_strict {
            true => arg_ty,
            false => syn::parse2(quote! {
                Option<#arg_ty>
            })
            .wrap_err("Wrapping argument type")?,
        };
        let arg_name = match arg_name {
            Some(name) if name.len() > 0 => name.clone(),
            _ => format!("arg{}", arg_idx),
        };
        let arg_ident: syn::Ident = syn::parse_str(&arg_name).wrap_err("Invalid ident")?;

        user_fn_arg_idents.push(arg_ident);
        user_fn_arg_types.push(arg_ty_wrapped);
    }

    let user_fn_return_type = oid_to_syn_type(return_type, true).wrap_err("Mapping return type")?;
    let user_fn_return_type_wrapped: syn::Type = match is_set {
        true => {
            syn::parse2(quote! { Option<impl Iterator<Item=Option<#user_fn_return_type>> + '_> })
                .wrap_err("Wrapping return type")?
        }
        false => {
            syn::parse2(quote! { Option<#user_fn_return_type> }).wrap_err("Wrapping return type")?
        }
    };

    file.items.push(
        syn::parse2(quote! {
            #[pg_extern]
            fn #user_fn_ident(
                #( #user_fn_arg_idents: #user_fn_arg_types ),*
            ) -> #user_fn_return_type_wrapped
            #user_code
        })
        .wrap_err("Parsing generated user function")?,
    );

    Ok(file)
}

#[tracing::instrument(level = "debug")]
fn extract_code_and_args(
    fn_oid: pg_sys::Oid,
) -> eyre::Result<(
    syn::Block,
    toml::value::Table,
    Vec<(PgOid, Option<String>)>,
    (PgOid, bool),
    bool,
)> {
    unsafe {
        let proc_tuple = pg_sys::SearchSysCache(
            pg_sys::SysCacheIdentifier_PROCOID as i32,
            fn_oid.into_datum().unwrap(),
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
        let plrust = std::ffi::CString::new("plrust").unwrap();
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

        let proc_entry = PgBox::from_pg(heap_tuple_get_struct::<pg_sys::FormData_pg_proc>(
            proc_tuple,
        ));

        let mut args = Vec::new();
        for i in 0..proc_entry.pronargs as usize {
            let type_oid = argtypes.get(i).expect("no type_oid for argument");
            let name = argnames.as_ref().and_then(|v| v.get(i).cloned()).flatten();

            args.push((PgOid::from(*type_oid), name));
        }

        let is_strict = proc_entry.proisstrict;
        let return_type = (PgOid::from(proc_entry.prorettype), proc_entry.proretset);

        pg_sys::ReleaseSysCache(proc_tuple);

        Ok((user_code, user_dependencies, args, return_type, is_strict))
    }
}

#[tracing::instrument(level = "debug", skip_all)]
fn parse_source_and_deps(code_and_deps: &str) -> Result<(syn::Block, toml::value::Table)> {
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
