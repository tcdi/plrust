/*
Portions Copyright 2020-2021 ZomboDB, LLC.
Portions Copyright 2021-2022 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

use crate::{error::PlRustError, gucs};
use color_eyre::section::{Section, SectionExt};
use eyre::{eyre, Result};
use libloading::{Library, Symbol};
use once_cell::unsync::Lazy;
use pgx::{pg_sys::heap_tuple_get_struct, *};
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
pub(crate) mod generation {
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
    enum Error {
        #[error("No generations found (Mac OS x86_64 specific)")]
        NoGenerations,
        #[error("std::io::Error: {0}")]
        StdIoError(#[from] std::io::Error),
    }

    /// Find existing generations of a given prefix.
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
    pub(crate) fn next_generation(prefix: &str, vacuum: bool) -> Result<usize, Error> {
        let latest = latest_generation(prefix, vacuum);
        Ok(latest.map(|this| this.0 + 1).unwrap_or_default())
    }

    /// Get the latest created generation night.
    ///
    /// If `vacuum` is set, this garbage collect old `so` files.
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

#[tracing::instrument(level = "info")]
pub(crate) unsafe fn unload_function(fn_oid: pg_sys::Oid) {
    LOADED_SYMBOLS.remove(&fn_oid);
}

#[tracing::instrument(level = "info")]
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

#[tracing::instrument(level = "info")]
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

    std::fs::create_dir_all(&crate_dir).map_err(PlRustError::CrateDirectory)?;

    let source_code = create_function_crate(fn_oid, &crate_dir, &crate_name)?;

    let cargo_output = Command::new("cargo")
        .current_dir(&crate_dir)
        .arg("build")
        .arg("--release")
        .env("PGX_PG_CONFIG_PATH", gucs::pg_config())
        .env("CARGO_TARGET_DIR", &work_dir)
        .env(
            "RUSTFLAGS",
            "-Ctarget-cpu=native -Clink-args=-Wl,-undefined,dynamic_lookup",
        )
        .output()
        .map_err(PlRustError::CargoBuildExec)?;

    let stdout = String::from_utf8(cargo_output.stdout).map_err(PlRustError::CargoOutputNotUtf8)?;
    let stderr = String::from_utf8(cargo_output.stderr).map_err(PlRustError::CargoOutputNotUtf8)?;

    let (final_path, stdout, stderr) = if !cargo_output.status.success() {
        return Err(eyre!(PlRustError::CargoBuildFail)
            .section(stdout.header("`cargo build` stdout:"))
            .section(stderr.header("`cargo build` stderr:"))
            .section(source_code.header("Source Code:")))?;
    } else {
        match find_shared_library(&crate_name).0 {
            Some(shared_library) => {
                let final_path = work_dir.join(&format!("{crate_name}{DLL_SUFFIX}"));

                // move the shared_library into its final location, which is
                // at the root of the configured `work_dir`
                std::fs::rename(&shared_library, &final_path)
                    .expect("unable to rename shared_library");

                (final_path, stdout, stderr)
            }
            None => return Err(PlRustError::SharedObjectNotFound)?,
        }
    };

    // no matter what happened, remove our crate directory, ignoring any error that might generate
    std::fs::remove_dir_all(&crate_dir).ok();

    Ok((final_path, stdout, stderr))
}

#[tracing::instrument(level = "info")]
fn create_function_crate(
    fn_oid: pg_sys::Oid,
    crate_dir: &PathBuf,
    crate_name: &str,
) -> Result<String, PlRustError> {
    let (fn_oid, deps, code, args, (return_type, is_set), is_strict) =
        extract_code_and_args(fn_oid)?;
    let source_code =
        generate_function_source(fn_oid, &code, &args, &return_type, is_set, is_strict)?;

    // cargo.toml first
    let cargo_toml = crate_dir.join("Cargo.toml");
    let major_version = pg_sys::get_pg_major_version_num();
    std::fs::write(
        &cargo_toml,
        &format!(
            r#"[package]
name = "{crate_name}"
version = "0.0.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[features]
default = ["pgx/pg{major_version}"]

[dependencies]
pgx = "0.4.3"
{deps}

[profile.release]
panic = "unwind"
opt-level = 3
lto = "fat"
codegen-units = 1
"#,
        ),
    )
    .map_err(PlRustError::WritingCargoToml)?;

    // the src/ directory
    let src = crate_dir.join("src");
    std::fs::create_dir_all(&src).map_err(PlRustError::CreatingSourceDirectory)?;

    // the actual source code in src/lib.rs
    let lib_rs = src.join("lib.rs");
    std::fs::write(&lib_rs, &source_code).map_err(PlRustError::WritingLibRs)?;

    Ok(source_code)
}

#[tracing::instrument(level = "info")]
fn crate_name(fn_oid: pg_sys::Oid) -> String {
    let db_oid = unsafe { pg_sys::MyDatabaseId };
    let ns_oid = unsafe { pg_sys::get_func_namespace(fn_oid) };
    format!("fn{}_{}_{}", db_oid, ns_oid, fn_oid)
}

#[tracing::instrument(level = "info")]
fn crate_name_and_path(fn_oid: pg_sys::Oid) -> (String, PathBuf) {
    let crate_name = crate_name(fn_oid);
    let crate_dir = gucs::work_dir().join(&crate_name);

    (crate_name, crate_dir)
}

#[tracing::instrument(level = "info")]
fn find_shared_library(crate_name: &str) -> (Option<PathBuf>, &str) {
    let target_dir = gucs::work_dir().join("release");
    let so = target_dir.join(&format!("lib{crate_name}{DLL_SUFFIX}"));

    if so.exists() {
        (Some(so), crate_name)
    } else {
        (None, crate_name)
    }
}

#[tracing::instrument(level = "info")]
fn generate_function_source(
    fn_oid: pg_sys::Oid,
    code: &str,
    args: &Vec<(PgOid, Option<String>)>,
    return_type: &PgOid,
    is_set: bool,
    is_strict: bool,
) -> Result<String, PlRustError> {
    let mut source = String::new();

    // source header
    source.push_str("\nuse pgx::*;\n");

    // function name
    source.push_str(&format!(
        r#"
#[pg_extern]
fn plrust_fn_{fn_oid}"#
    ));

    // function args
    source.push('(');
    for (idx, (type_oid, name)) in args.iter().enumerate() {
        if idx > 0 {
            source.push_str(", ");
        }

        let mut rust_type = make_rust_type(type_oid, false)
            .ok_or(PlRustError::UnsupportedSqlType(type_oid.value()))?
            .to_string();

        if !is_strict {
            // non-STRICT functions need all arguments as an Option<T> as any of them could be NULL
            rust_type = format!("Option<{}>", rust_type);
        }

        match name {
            Some(name) if name.len() > 0 => source.push_str(&format!("{}: {}", name, rust_type)),
            _ => source.push_str(&format!("arg{}: {}", idx + 1, rust_type)),
        }
    }
    source.push(')');

    // return type
    source.push_str(" -> ");
    let ret = make_rust_type(return_type, true)
        .ok_or(PlRustError::UnsupportedSqlType(return_type.value()))?;
    if is_set {
        source.push_str(&format!("impl std::iter::Iterator<Item = Option<{ret}>>"));
    } else {
        source.push_str(&format!("Option<{ret}>"));
    }

    // body
    source.push_str(" {\n");
    source.push_str(&code);
    source.push_str("\n}");
    Ok(source)
}

#[tracing::instrument(level = "info")]
fn extract_code_and_args(
    fn_oid: pg_sys::Oid,
) -> Result<
    (
        pg_sys::Oid,
        String,
        String,
        Vec<(PgOid, Option<String>)>,
        (PgOid, bool),
        bool,
    ),
    PlRustError,
> {
    unsafe {
        let proc_tuple = pg_sys::SearchSysCache(
            pg_sys::SysCacheIdentifier_PROCOID as i32,
            fn_oid.into_datum().unwrap(),
            0,
            0,
            0,
        );
        if proc_tuple.is_null() {
            return Err(PlRustError::NullProcTuple);
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
            return Err(PlRustError::NotPlRustFunction(fn_oid));
        }

        let prosrc_datum = pg_sys::SysCacheGetAttr(
            pg_sys::SysCacheIdentifier_PROCOID as i32,
            proc_tuple,
            pg_sys::Anum_pg_proc_prosrc as pg_sys::AttrNumber,
            &mut is_null,
        );
        let (deps, source_code) = parse_source_and_deps(
            &String::from_datum(prosrc_datum, is_null, pg_sys::TEXTOID)
                .ok_or(PlRustError::NullSourceCode)?,
        );
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

        Ok((fn_oid, deps, source_code, args, return_type, is_strict))
    }
}

#[tracing::instrument(level = "info")]
fn parse_source_and_deps(code: &str) -> (String, String) {
    enum Parse {
        Code,
        Deps,
    }
    let mut deps_block = String::new();
    let mut code_block = String::new();
    let mut parse = Parse::Code;

    for line in code.trim().split_inclusive('\n') {
        match line.trim() {
            "[dependencies]" => parse = Parse::Deps,
            "[code]" => parse = Parse::Code,
            _ => match parse {
                Parse::Code => code_block.push_str(line),
                Parse::Deps => deps_block.push_str(line),
            },
        }
    }

    (deps_block, code_block)
}

#[tracing::instrument(level = "info")]
fn make_rust_type(type_oid: &PgOid, owned: bool) -> Option<String> {
    let array_type = unsafe { pg_sys::get_element_type(type_oid.value()) };
    let array = array_type != pg_sys::InvalidOid;
    let type_oid = if array {
        PgOid::from(array_type)
    } else {
        *type_oid
    };

    let rust_type = match type_oid {
        PgOid::BuiltIn(builtin) => match builtin {
            PgBuiltInOids::ANYELEMENTOID => "AnyElement",
            PgBuiltInOids::BOOLOID => "bool",
            PgBuiltInOids::BYTEAOID if owned => "Vec<Option<[u8]]>>",
            PgBuiltInOids::BYTEAOID => "&[u8]",
            PgBuiltInOids::CHAROID => "u8",
            PgBuiltInOids::CSTRINGOID => "std::ffi::CStr",
            PgBuiltInOids::FLOAT4OID => "f32",
            PgBuiltInOids::FLOAT8OID => "f64",
            PgBuiltInOids::INETOID => "Inet",
            PgBuiltInOids::INT2OID => "i16",
            PgBuiltInOids::INT4OID => "i32",
            PgBuiltInOids::INT8OID => "i64",
            PgBuiltInOids::JSONBOID => "JsonB",
            PgBuiltInOids::JSONOID => "Json",
            PgBuiltInOids::NUMERICOID => "Numeric",
            PgBuiltInOids::OIDOID => "pg_sys::Oid",
            PgBuiltInOids::TEXTOID if owned => "String",
            PgBuiltInOids::TEXTOID => "&str",
            PgBuiltInOids::TIDOID => "pg_sys::ItemPointer",
            PgBuiltInOids::VARCHAROID if owned => "String",
            PgBuiltInOids::VARCHAROID => "&str",
            PgBuiltInOids::VOIDOID => "()",
            _ => return None,
        },
        _ => return None,
    }
    .to_string();

    Some(if array && owned {
        format!("Vec<Option<{rust_type}>>")
    } else if array {
        format!("Array<{rust_type}>")
    } else {
        rust_type
    })
}
