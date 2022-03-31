/*
Portions Copyright 2020-2021 ZomboDB, LLC.
Portions Copyright 2021-2022 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

use crate::gucs;
use pgx::pg_sys::heap_tuple_get_struct;
use pgx::*;
use wasmtime::{Val, ValType};
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
    (
        Module,
        Vec<PgOid>, // Arg OIDs
        Vec<Val>,   // Arg value slots
        PgOid, // Return OIDs
        Val,   // Return value slots
    ),
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

fn initialize_cache_entry(fn_oid: pg_sys::Oid, fcinfo: pg_sys::FunctionCallInfo) -> (
    Module,
    Vec<PgOid>, // Arg OIDs
    Vec<Val>,   // Arg value slots
    PgOid, // Return OIDs
    Val,   // Return value slots
) {
    let wasm_fn_name = format!("plrust_fn_{}", fn_oid);
    let (crate_name, crate_dir) = crate_name_and_path(fn_oid);
    let wasm = format!("{}.wasm", crate_dir.to_str().unwrap());

    let module = match Module::from_file(&ENGINE, wasm) {
        Ok(m) => m,
        Err(e) => panic!(
            "Could not set up module {}.wasm from directory {:#?}: {}",
            crate_name, crate_dir, e
        ),
    };
    let (argtypes, rettype) = unsafe {
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
        let argtypes_datum = pg_sys::SysCacheGetAttr(
            pg_sys::SysCacheIdentifier_PROCOID as i32,
            proc_tuple,
            pg_sys::Anum_pg_proc_proargtypes as pg_sys::AttrNumber,
            &mut is_null,
        );
        let argtypes = Vec::<pg_sys::Oid>::from_datum(argtypes_datum, is_null, pg_sys::OIDARRAYOID).unwrap()
            .iter()
            .map(|&v| PgOid::from(v))
            .collect::<Vec<_>>();
        
        let proc_entry = PgBox::from_pg(heap_tuple_get_struct::<pg_sys::FormData_pg_proc>(
            proc_tuple,
        ));
        let rettype = PgOid::from(proc_entry.prorettype);

        // Make **sure** we have a copy as we're about to release it.
        pg_sys::ReleaseSysCache(proc_tuple);
        (argtypes, rettype)
    };

    let mut args: Vec<wasmtime::Val> = vec![ ];
    for idx in 0..argtypes.len() {
        args.push(Val::ExternRef(None));
    }
    let mut ret = Val::ExternRef(None);

    (module, argtypes, args, rettype, ret)
}

pub(crate) unsafe fn execute_wasm_function(fn_oid: pg_sys::Oid, fcinfo: pg_sys::FunctionCallInfo) -> pg_sys::Datum {
    let wasm_fn_name = format!("plrust_fn_{}", fn_oid);
    let (module, argtypes, args, rettypes, rets) = CACHE.entry(fn_oid).or_insert_with(|| 
        initialize_cache_entry(fn_oid, fcinfo)
    );

    let mut store = Store::new(&ENGINE, WasiCtxBuilder::new().inherit_stdio().build());

    let instance = match LINKER.instantiate(&mut store, &module) {
        Ok(i) => i,
        Err(e) => panic!(
            "Could not instantiate {}: {}",
            wasm_fn_name, e
        ),
    };

    let mut instance_args = args.clone();
    for (idx, val) in instance_args.iter_mut().enumerate() {
        pgx::log!("Got OID {:?}", argtypes[idx]);
        let wasm_val = match oid_to_valtype(&argtypes[idx]) {
            Some(valtype) => match valtype {
                ValType::I32 => Val::I32(pg_getarg(fcinfo, idx).unwrap()),
                ValType::I64 => Val::I64(pg_getarg(fcinfo, idx).unwrap()),
                ValType::F32 => todo!(),
                ValType::F64 => todo!(),
                ValType::V128 => todo!(),
                ValType::FuncRef => todo!(),
                ValType::ExternRef => todo!(),
            },
            None => {
                let datum = pg_getarg_datum(fcinfo, idx).unwrap();

                let bincoded = match &argtypes[idx] {
                    PgOid::InvalidOid => todo!(),
                    PgOid::Custom(_) => todo!(),
                    PgOid::BuiltIn(builtin) => match builtin {
                        PgBuiltInOids::TEXTOID => bincode::serialize(&pg_getarg::<String>(fcinfo, idx).unwrap()).unwrap(),
                        _ => todo!(),
                    },
                };

                let wasm_alloc = match instance.get_typed_func::<(u64, u64), u64, _>(&mut store, &"guest_alloc") {
                    Ok(f) => f,
                    Err(e) => panic!("Could not find function {}: {}", wasm_fn_name, e),
                };
                let wasm_dealloc = match instance.get_typed_func::<(u64, u64, u64), (), _>(&mut store, &"guest_dealloc") {
                    Ok(f) => f,
                    Err(e) => panic!("Could not find function {}: {}", wasm_fn_name, e),
                };

                pgx::info!("About to alloc {} bytes", bincoded.len());
                let guest_ptr = match wasm_alloc.call(&mut store, (bincoded.len() as u64, 8)) {
                    Ok(res) => res,
                    Err(e) => panic!("Got an error: {:?}", e),
                };
                pgx::info!("Wrote {} bytes at offset {}", bincoded.len(), guest_ptr);
                instance.get_memory(&mut store, "memory").unwrap().write(&mut store, guest_ptr as usize, bincoded.as_slice()).unwrap();

                let packed = plrust_interface::pack_and_leak_into_wasm_u128(bincoded);
                Val::V128(packed)
            },
        };

        *val = wasm_val;
    }
    let mut instance_ret = [ rets.clone() ];

    let wasm_fn = match instance.get_func(&mut store, &"entry") {
        Some(f) => f,
        None => panic!("Could not find function {}", wasm_fn_name),
    };

    match wasm_fn.call(&mut store, instance_args.as_slice(), instance_ret.as_mut_slice()) {
        Ok(res) => res,
        Err(e) => panic!("Got an error: {:?}", e),
    };

    let res = match &instance_ret[0] {
        wasmtime::Val::V128(val) => val,
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

    let source_code_formatted = prettyplease::unparse(&source_code);
    std::fs::write(&lib_rs, &source_code_formatted).expect("failed to write source code to lib.rs");

    source_code_formatted
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
) -> syn::File {
    let mut source = syn::File {
        shebang: Default::default(),
        attrs: Default::default(),
        items: Default::default(),
    };

    // User defined function
    let user_fn_name = &format!("plrust_fn_{}", fn_oid);
    let user_fn_ident = syn::Ident::new(user_fn_name, proc_macro2::Span::call_site());
    let mut user_fn_arg_idents: Vec<syn::Ident> = Vec::default(); 
    let mut user_fn_arg_types: Vec<syn::Type> = Vec::default();
    for (arg_idx, (arg_type_oid, arg_name)) in args.iter().enumerate() {
        let arg_ty = oid_to_syn_type(arg_type_oid, false).unwrap();
        let arg_name = match arg_name {
            Some(name) if name.len() > 0 => name.clone(),
            _ => format!("arg{}", arg_idx),
        };
        let arg_ident: syn::Ident = syn::parse_str(&arg_name).expect("Invalid ident");

        user_fn_arg_idents.push(arg_ident);
        user_fn_arg_types.push(arg_ty);
    }
    let user_fn_block_tokens: syn::Block = syn::parse_str(&format!("{{ {} }}", code)).expect("Couldn't parse user code");
    let user_fn_return_tokens = oid_to_syn_type(return_type, true);

    let user_fn_tokens: syn::ItemFn = syn::parse_quote! {
        fn #user_fn_ident(
            #( #user_fn_arg_idents: #user_fn_arg_types ),*
        ) -> #user_fn_return_tokens
        #user_fn_block_tokens
    };
    source.items.push(syn::Item::Fn(user_fn_tokens));

    let entry_fn_arg_idents = user_fn_arg_idents.clone();
    let mut entry_fn_arg_types: Vec<syn::Type> = Vec::default();
    let mut entry_fn_transform_tokens: Vec<syn::Expr> = Vec::default();
    for (arg_idx, (arg_type_oid, arg_name)) in args.iter().enumerate() {
        let (mapped, is_u128_ptr) = match oid_to_valtype(arg_type_oid) {
            Some(valtype) => {
                // It's a primitive, we pass directly.
                let ty = valtype_to_syn_type(valtype).unwrap();
                (syn::parse_quote! { #ty }, false)
            },
            None => {
                // It's an encoded value. This expands to (ptr, len)
                (syn::parse_quote! { u128 }, true)
            },
        };
        entry_fn_arg_types.push(mapped);

        let ident = &user_fn_arg_idents[arg_idx];
        entry_fn_transform_tokens.push(match is_u128_ptr {
            true => syn::parse_quote! { unsafe { ::plrust_interface::unpack_and_own_from_wasm_u128(#ident).unwrap() } },
            false => syn::parse_quote! { #ident },
        })
    }
    let entry_fn_return_tokens = match oid_to_valtype(return_type) {
        Some(valtype) => {
            // It's a primitive, we pass directly.
            valtype_to_syn_type(valtype).unwrap()
        },
        None => {
            // It's an encoded value. This expands to (ptr, len)
            syn::parse_quote! { u128 }
        },
    };

    let entry_fn: syn::ItemFn = syn::parse_quote! {
        #[no_mangle]
        extern "C" fn entry(
            #( #entry_fn_arg_idents: #entry_fn_arg_types ),*
        ) -> #entry_fn_return_tokens {
            let retval = #user_fn_ident(
                #(#entry_fn_transform_tokens),*
            );
            unsafe { plrust_interface::serialize_and_leak_into_wasm_u128(&retval).unwrap() }
        }
    };
    source.items.push(syn::Item::Fn(entry_fn));
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

fn oid_to_valtype(oid: &pg_sys::PgOid) -> Option<ValType> {
    match oid {
        PgOid::InvalidOid => todo!(),
        PgOid::Custom(_) => todo!(),
        PgOid::BuiltIn(builtin) => match builtin {
            PgBuiltInOids::INT4OID => Some(ValType::I32),
            PgBuiltInOids::INT8OID => Some(ValType::I64),
            _ => None,
        },
    }
}

fn valtype_to_oid(valtype: ValType) -> Option<PgOid> {
    match valtype {
        ValType::I32 => Some(PgOid::BuiltIn(PgBuiltInOids::INT4OID)),
        ValType::I64 => Some(PgOid::BuiltIn(PgBuiltInOids::INT8OID)),
        ValType::F32 => todo!(),
        ValType::F64 => todo!(),
        ValType::V128 => todo!(),
        ValType::FuncRef => todo!(),
        ValType::ExternRef => todo!(),
    }
}

fn valtype_to_syn_type(valtype: ValType) -> Option<syn::Type> {
    match valtype {
        ValType::I32 => Some(syn::parse_quote! { i32 }),
        ValType::I64 => Some(syn::parse_quote! { i64 }),
        ValType::F32 => todo!(),
        ValType::F64 => todo!(),
        ValType::V128 => todo!(),
        ValType::FuncRef => todo!(),
        ValType::ExternRef => todo!(),
    }
}

fn oid_to_syn_type(type_oid: &PgOid, owned: bool) -> Option<syn::Type> {
    let array_type = unsafe { pg_sys::get_element_type(type_oid.value()) };

    let (base_oid, array) = if array_type != pg_sys::InvalidOid {
        (PgOid::from(array_type), true)
    } else {
        (type_oid.clone(), false)
    };

    let mut base_rust_type: syn::Type = match base_oid {
        PgOid::BuiltIn(builtin) => match builtin {
            PgBuiltInOids::ANYELEMENTOID => syn::parse_quote! { AnyElement },
            PgBuiltInOids::BOOLOID => syn::parse_quote! { bool },
            PgBuiltInOids::BYTEAOID if owned => syn::parse_quote! { Vec<Option<[u8]>> },
            PgBuiltInOids::BYTEAOID => syn::parse_quote! { &[u8] },
            PgBuiltInOids::CHAROID => syn::parse_quote! { u8 },
            PgBuiltInOids::CSTRINGOID => syn::parse_quote! { std::ffi::CStr },
            PgBuiltInOids::FLOAT4OID => syn::parse_quote! { f32 },
            PgBuiltInOids::FLOAT8OID => syn::parse_quote! { f64 },
            PgBuiltInOids::INETOID => syn::parse_quote! { Inet },
            PgBuiltInOids::INT2OID => syn::parse_quote! { i16 },
            PgBuiltInOids::INT4OID => syn::parse_quote! { i32 },
            PgBuiltInOids::INT8OID => syn::parse_quote! { i64 },
            PgBuiltInOids::JSONBOID => syn::parse_quote! { JsonB },
            PgBuiltInOids::JSONOID => syn::parse_quote! { Json },
            PgBuiltInOids::NUMERICOID => syn::parse_quote! { Numeric },
            PgBuiltInOids::OIDOID => syn::parse_quote! { pg_sys::Oid },
            PgBuiltInOids::TEXTOID if owned => syn::parse_quote! { String },
            PgBuiltInOids::TEXTOID => syn::parse_quote! { &str },
            PgBuiltInOids::TIDOID => syn::parse_quote! { pg_sys::ItemPointer },
            PgBuiltInOids::VARCHAROID if owned => syn::parse_quote! { String },
            PgBuiltInOids::VARCHAROID => syn::parse_quote! { &str },
            PgBuiltInOids::VOIDOID => syn::parse_quote! { () },
            _ => return None,
        },
        _ => return None,
    };
    
    if array && owned {
        Some(syn::parse_quote! { Vec<Option<#base_rust_type>> })
    } else if array {
        Some(syn::parse_quote! { Array<#base_rust_type> })
    } else {
        Some(base_rust_type)
    }
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
