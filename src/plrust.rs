/*
Portions Copyright 2020-2021 ZomboDB, LLC.
Portions Copyright 2021-2022 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

use crate::gucs;
use pgx::pg_sys::heap_tuple_get_struct;
use pgx::*;
use std::{path::PathBuf, collections::HashMap, process::Command, io::Write};

use wasmtime::{Engine, Linker, Store, Module};
use wasmtime_wasi::{WasiCtx, sync::WasiCtxBuilder};

use once_cell::sync::Lazy;

static ENGINE: Lazy<Engine> = Lazy::new(|| Engine::default());
static LINKER: Lazy<Linker<WasiCtx>> = Lazy::new(|| {
    let mut linker = Linker::new(&ENGINE);

    match wasmtime_wasi::add_to_linker(&mut linker, |cx| cx) {
        Ok(_) => {}
        Err(_) => panic!("failed to call add_to_linker"),
    };

    plrust_interface::create_linker_functions(&mut linker)
        .expect("Could not create linker functions");

    linker
});
static mut CACHE: Lazy<HashMap<
    pg_sys::Oid,
    (Module, i64, i64),
>> = Lazy::new(|| Default::default());

static INTERFACE_CRATE_PATH: Lazy<PathBuf> = Lazy::new(|| {
    let mut interface_crate = gucs::work_dir();
    interface_crate.push("plrust_interface");
    interface_crate
});
static INTERFACE_CRATE: include_dir::Dir<'_> = include_dir::include_dir!("$CARGO_MANIFEST_DIR/components/plrust_interface");

pub(crate) fn init() {
    provision_interface_crate(&INTERFACE_CRATE)
}

fn provision_interface_crate(dir: &include_dir::Dir) {
    for entry in dir.entries() {
        match entry {
            include_dir::DirEntry::File(entry_file) => {
                let mut file_destination = INTERFACE_CRATE_PATH.clone();
                file_destination.push(entry_file.path());

                std::fs::create_dir_all(file_destination.parent().unwrap()).unwrap();
                let mut destination = std::fs::File::create(file_destination).unwrap();
                destination.write_all(entry_file.contents()).unwrap();
            }
            include_dir::DirEntry::Dir(dir) => provision_interface_crate(dir),
        }
    }
}

pub(crate) unsafe fn execute_wasm_function(fn_oid: pg_sys::Oid, fcinfo: pg_sys::FunctionCallInfo) -> pg_sys::Datum {
    let wasm_fn_name = format!("plrust_fn_{}", fn_oid);
    let (crate_name, crate_dir) = crate_name_and_path(fn_oid);

    let wasm = format!("{}.wasm", crate_dir.to_str().unwrap());

   let (module, n_args, n_retvals) = CACHE.entry(fn_oid).or_insert_with(|| {
        let module = match Module::from_file(&ENGINE, wasm) {
            Ok(m) => m,
            Err(e) => panic!(
                "Could not set up module {}.wasm from directory {:#?}: {}",
                crate_name, crate_dir, e
            ),
        };
        let n_args = unsafe {
            let proc_tuple = pg_sys::SearchSysCache(
                pg_sys::SysCacheIdentifier_PROCOID as i32,
                fn_oid.into_datum().unwrap(),
                0,
                0,
                0,
            );
            if proc_tuple.is_null() {
                panic!("cache lookup failed for function oid {}", fn_oid);
            }
    
            let proc_entry = PgBox::from_pg(heap_tuple_get_struct::<pg_sys::FormData_pg_proc>(
                proc_tuple,
            ));
    
            let n_args = proc_entry.pronargs as i64;
            pg_sys::ReleaseSysCache(proc_tuple);
            n_args
        };
        (module, n_args, 1)
    });

    let mut store = Store::new(&ENGINE, WasiCtxBuilder::new().inherit_stdio().build());

    let instance = match LINKER.instantiate(&mut store, &module) {
        Ok(i) => i,
        Err(e) => panic!(
            "Could not instantiate instance from module {}.wasm: {}",
            crate_name, e
        ),
    };
    let wasm_fn = match instance.get_func(&mut store, &wasm_fn_name) {
        Some(f) => f,
        None => panic!("Could not find function {}", wasm_fn_name),
    };

    let mut args: Vec<wasmtime::Val> = vec![ ];
    for idx in 0..(*n_args) {
        args.push(wasmtime::Val::I32(pg_getarg(fcinfo, idx as usize).unwrap()));
    }
    let mut returns: Vec<wasmtime::Val> = vec![ ];
    for idx in 0..(*n_retvals) {
        returns.push(wasmtime::Val::I32(0));
    }

    match wasm_fn.call(&mut store, args.as_slice(), returns.as_mut_slice()) {
        Ok(res) => res,
        Err(e) => panic!("Got an error: {:?}", e),
    };

    let res = match &returns[0] {
        wasmtime::Val::I32(val) => val,
        other => unimplemented!("Cannot handle {:?}", other),
    };
    *res as pg_sys::Datum
}

pub(crate) unsafe fn unload_function(fn_oid: pg_sys::Oid) {
    CACHE.remove(&fn_oid);
}

pub(crate) fn compile_function(fn_oid: pg_sys::Oid) -> Result<(PathBuf, String), String> {
    let work_dir = gucs::work_dir();
    let (crate_name, crate_dir) = crate_name_and_path(fn_oid);

    std::fs::create_dir_all(&crate_dir).expect("failed to create crate directory");

    let source_code = create_function_crate(fn_oid, &crate_dir, &crate_name);

    let wasm_build_output = Command::new("cargo")
        .current_dir(&crate_dir)
        .arg("build")
        .arg("--target")
        .arg("wasm32-wasi")
        .arg("--release")
        .output()
        .expect("failed to build function wasm module");

    let mut wasm_build_output_string = String::new();
    unsafe {
        wasm_build_output_string.push_str(&String::from_utf8_unchecked(wasm_build_output.stdout));
        wasm_build_output_string.push_str(&String::from_utf8_unchecked(wasm_build_output.stderr));
    }

    let result = if !wasm_build_output.status.success() {
        wasm_build_output_string.push_str("-----------------\n");
        wasm_build_output_string.push_str(&source_code);
        Err(wasm_build_output_string)
    } else {
        match find_wasm_module(&crate_name) {
            Some(wasm_module) => {
                pgx::info!("{}", crate_name);
                let mut final_path = work_dir.clone();
                final_path.push(&format!("{}.wasm", crate_name));

                // move the wasm module into its final location, which is
                // at the root of the configured `work_dir`
                std::fs::rename(&wasm_module, &final_path).expect("unable to rename wasm module");

                Ok((final_path, wasm_build_output_string))
            }
            None => Err(wasm_build_output_string),
        }
    };

    // Let's keep the crate for debugging purpose
    // std::fs::remove_dir_all(&crate_dir).ok(); 

    result
}

fn create_function_crate(fn_oid: pg_sys::Oid, crate_dir: &PathBuf, crate_name: &str) -> String {
    let (fn_oid, dependencies, code, args, (return_type, is_set), is_strict) =
        extract_code_and_args(fn_oid);
    let source_code =
        generate_function_source(fn_oid, &code, &args, &return_type, is_set, is_strict);

    // cargo.toml first
    let mut cargo_toml = crate_dir.clone();
    cargo_toml.push("Cargo.toml");
    std::fs::write(
        &cargo_toml,
        &format!(
            r#"[package]
name = "{crate_name}"
version = "0.0.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
plrust_interface = {{ path = "{plrust_interface_crate_path}" }}
{dependencies}
"#,
            crate_name = crate_name,
            dependencies = dependencies,
            plrust_interface_crate_path = INTERFACE_CRATE_PATH.display(),
        ),
    )
    .expect("failed to write Cargo.toml");

    // the src/ directory
    let mut src = crate_dir.clone();
    src.push("src");
    std::fs::create_dir_all(&src).expect("failed to create src directory");

    // the actual source code in src/lib.rs
    let mut lib_rs = src.clone();
    lib_rs.push("lib.rs");
    std::fs::write(&lib_rs, &source_code).expect("failed to write source code to lib.rs");

    source_code
}

fn crate_name(fn_oid: pg_sys::Oid) -> String {
    let db_oid = unsafe { pg_sys::MyDatabaseId };
    let ns_oid = unsafe { pg_sys::get_func_namespace(fn_oid) };
    format!("fn{}_{}_{}", db_oid, ns_oid, fn_oid)
}

fn crate_name_and_path(fn_oid: pg_sys::Oid) -> (String, PathBuf) {
    let mut crate_dir = gucs::work_dir();
    let crate_name = crate_name(fn_oid);
    crate_dir.push(&crate_name);

    (crate_name, crate_dir)
}

fn find_wasm_module(crate_name: &str) -> Option<PathBuf> {
    let work_dir = gucs::work_dir();
    let mut debug_dir = work_dir.clone();
    debug_dir.push(&crate_name);
    debug_dir.push("target");
    debug_dir.push("wasm32-wasi");
    debug_dir.push("release");

    let mut wasm = debug_dir.clone();
    wasm.push(&format!("{}.wasm", crate_name));
    if wasm.exists() {
        return Some(wasm);
    }

    None
}

fn generate_function_source(
    fn_oid: pg_sys::Oid,
    code: &str,
    args: &Vec<(PgOid, Option<String>)>,
    return_type: &PgOid,
    is_set: bool,
    is_strict: bool,
) -> String {
    let mut source = String::new();

    // function name
    source.push_str(&format!(
        r#"
#[no_mangle]
fn plrust_fn_{}"#,
        fn_oid
    ));

    // function args
    source.push('(');
    for (idx, (type_oid, name)) in args.iter().enumerate() {
        if idx > 0 {
            source.push_str(", ");
        }

        let mut rust_type = make_rust_type(type_oid, false).to_string();

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
    if is_set {
        source.push_str(&format!(
            "impl std::iter::Iterator<Item = Option<{}>>",
            make_rust_type(return_type, true)
        ));
    } else {
        // wasmtime does not handle option type quite well, so we'll assume everything without option
        source.push_str(make_rust_type(return_type, true).as_str());
    }

    // body
    source.push_str(" {\n");
    source.push_str(&code);
    source.push_str("\n}");
    source
}

fn extract_code_and_args(
    fn_oid: pg_sys::Oid,
) -> (
    pg_sys::Oid,
    String,
    String,
    Vec<(PgOid, Option<String>)>,
    (PgOid, bool),
    bool,
) {
    unsafe {
        let proc_tuple = pg_sys::SearchSysCache(
            pg_sys::SysCacheIdentifier_PROCOID as i32,
            fn_oid.into_datum().unwrap(),
            0,
            0,
            0,
        );
        if proc_tuple.is_null() {
            panic!("cache lookup failed for function oid {}", fn_oid);
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
            panic!("function {} is not a plrust function", fn_oid);
        }

        let prosrc_datum = pg_sys::SysCacheGetAttr(
            pg_sys::SysCacheIdentifier_PROCOID as i32,
            proc_tuple,
            pg_sys::Anum_pg_proc_prosrc as pg_sys::AttrNumber,
            &mut is_null,
        );
        let (deps, source_code) = parse_source_and_deps(
            &String::from_datum(prosrc_datum, is_null, pg_sys::TEXTOID)
                .expect("source code was null"),
        );
        let argnames_datum = pg_sys::SysCacheGetAttr(
            pg_sys::SysCacheIdentifier_PROCOID as i32,
            proc_tuple,
            pg_sys::Anum_pg_proc_proargnames as pg_sys::AttrNumber,
            &mut is_null,
        );
        let argnames =
            Vec::<Option<String>>::from_datum(argnames_datum, is_null, pg_sys::TEXTARRAYOID);

        let argtypes_datum = pg_sys::SysCacheGetAttr(
            pg_sys::SysCacheIdentifier_PROCOID as i32,
            proc_tuple,
            pg_sys::Anum_pg_proc_proargtypes as pg_sys::AttrNumber,
            &mut is_null,
        );
        let argtypes = Vec::<pg_sys::Oid>::from_datum(argtypes_datum, is_null, pg_sys::OIDARRAYOID);

        let proc_entry = PgBox::from_pg(heap_tuple_get_struct::<pg_sys::FormData_pg_proc>(
            proc_tuple,
        ));

        let mut args = Vec::new();
        for i in 0..proc_entry.pronargs as usize {
            let type_oid = if argtypes.is_some() {
                argtypes.as_ref().unwrap().get(i)
            } else {
                None
            };
            let name = if argnames.is_some() {
                argnames.as_ref().unwrap().get(i).cloned().flatten()
            } else {
                None
            };

            args.push((
                PgOid::from(*type_oid.expect("no type_oid for argument")),
                name,
            ));
        }

        let is_strict = proc_entry.proisstrict;
        let return_type = (PgOid::from(proc_entry.prorettype), proc_entry.proretset);

        pg_sys::ReleaseSysCache(proc_tuple);

        (fn_oid, deps, source_code, args, return_type, is_strict)
    }
}

fn parse_source_and_deps(code: &str) -> (String, String) {
    let mut deps_block = String::new();
    let mut code_block = String::new();
    let mut in_deps = false;
    let mut in_code = true;

    for line in code.trim().lines() {
        let trimmed_line = line.trim();
        if trimmed_line == "[dependencies]" {
            // parsing deps
            in_deps = true;
            in_code = false;
        } else if trimmed_line == "[code]" {
            // parsing code
            in_deps = false;
            in_code = true;
        } else if in_deps {
            // track our dependencies
            deps_block.push_str(line);
            deps_block.push_str("\n");
        } else if in_code {
            // track our code
            code_block.push_str(line);
            code_block.push_str("\n");
        } else {
            panic!("unexpected pl/rust code state")
        }
    }

    (deps_block, code_block)
}

fn make_rust_type(type_oid: &PgOid, owned: bool) -> String {
    let array_type = unsafe { pg_sys::get_element_type(type_oid.value()) };

    let (base_oid, array) = if array_type != pg_sys::InvalidOid {
        (PgOid::from(array_type), true)
    } else {
        (type_oid.clone(), false)
    };

    let type_oid = base_oid;
    let mut rust_type = match type_oid {
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
            _ => panic!("unsupported argument type: {:?}", type_oid),
        },
        _ => panic!("unsupported argument type: {:?}", type_oid),
    }
    .to_string();

    if array && owned {
        rust_type = format!("Vec<Option<{}>>", rust_type);
    } else if array {
        rust_type = format!("Array<{}>", rust_type);
    }

    rust_type
}
