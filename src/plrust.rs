/*
Portions Copyright 2020-2021 ZomboDB, LLC.
Portions Copyright 2021-2022 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

use crate::gucs;
use pgx::pg_sys::heap_tuple_get_struct;
use pgx::*;
use std::{collections::HashMap, io::BufReader, path::PathBuf, process::Command};

use wasmtime::{Engine, Linker, Module, Store};
use wasmtime_wasi::{sync::WasiCtxBuilder, WasiCtx};

use crate::error::PlRustError;
use color_eyre::{Section, SectionExt};
use eyre::eyre;
use once_cell::sync::Lazy;

struct PlRustStore {
    wasi: WasiCtx,
    host: crate::interface::Host,
    guest_data: crate::guest::GuestData,
}

impl Default for PlRustStore {
    fn default() -> Self {
        Self {
            wasi: WasiCtxBuilder::new().inherit_stdio().build(),
            guest_data: crate::guest::GuestData::default(),
            host: crate::interface::Host::default(),
        }
    }
}

static ENGINE: Lazy<Engine> = Lazy::new(|| Engine::default());

static mut CACHE: Lazy<
    HashMap<
        pg_sys::Oid,
        (
            Module,
            Vec<PgOid>, // Arg OIDs
            PgOid,      // Return OIDs
        ),
    >,
> = Lazy::new(|| Default::default());

static WORK_DIR_GUEST_WIT: Lazy<PathBuf> = Lazy::new(|| {
    let mut guest_wit = gucs::work_dir();
    guest_wit.push("wit");
    guest_wit.push("guest.wit");
    guest_wit
});

static WORK_DIR_HOST_WIT: Lazy<PathBuf> = Lazy::new(|| {
    let mut host_wit = gucs::work_dir();
    host_wit.push("wit");
    host_wit.push("host.wit");
    host_wit
});

static WORK_DIR_VALUE_WIT: Lazy<PathBuf> = Lazy::new(|| {
    let mut host_wit = gucs::work_dir();
    host_wit.push("wit");
    host_wit.push("types.wit");
    host_wit
});

pub(crate) fn init() {
    let mut host_wit = gucs::work_dir();
    host_wit.push("wit");
    std::fs::create_dir_all(host_wit).unwrap();
    std::fs::write(&*WORK_DIR_GUEST_WIT, include_str!("wit/guest.wit")).unwrap();
    std::fs::write(&*WORK_DIR_HOST_WIT, include_str!("wit/host.wit")).unwrap();
    std::fs::write(&*WORK_DIR_VALUE_WIT, include_str!("wit/types.wit")).unwrap();
}

fn initialize_cache_entry(
    fn_oid: pg_sys::Oid,
) -> (
    Module,
    Vec<PgOid>, // Arg OIDs
    PgOid,      // Return OIDs
) {
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
        let argtypes = Vec::<pg_sys::Oid>::from_datum(argtypes_datum, is_null, pg_sys::OIDARRAYOID)
            .unwrap()
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

    (module, argtypes, rettype)
}

fn build_arg(
    idx: usize,
    oid: PgOid,
    fcinfo: pg_sys::FunctionCallInfo,
) -> crate::guest::ValueParam<'static> {
    use crate::guest::ValueParam;
    match oid {
        PgOid::BuiltIn(builtin) => match builtin {
            PgBuiltInOids::TEXTOID => ValueParam::String(pg_getarg(fcinfo, idx).unwrap()),
            _ => todo!(),
        },
        _ => todo!(),
    }
}

pub(crate) unsafe fn execute_wasm_function(
    fn_oid: pg_sys::Oid,
    fcinfo: pg_sys::FunctionCallInfo,
) -> eyre::Result<pg_sys::Datum> {
    let (module, arg_oids, _ret_oid) = CACHE
        .entry(fn_oid)
        .or_insert_with(|| initialize_cache_entry(fn_oid));

    let mut store = Store::new(&ENGINE, PlRustStore::default());

    let mut linker = Linker::new(&ENGINE);
    wasmtime_wasi::add_to_linker(&mut linker, |cx: &mut PlRustStore| &mut cx.wasi)
        .map_err(|e| eyre!(e))?;
    crate::host::add_to_linker(&mut linker, |cx: &mut PlRustStore| &mut cx.host)
        .map_err(|e| eyre!(e))?;

    let (guest, _guest_instance) =
        crate::guest::Guest::instantiate(&mut store, &module, &mut linker, |cx| &mut cx.guest_data)
            .map_err(|e| eyre!(e))?;

    let args = arg_oids
        .iter()
        .enumerate()
        .map(|(idx, arg_oid)| build_arg(idx, *arg_oid, fcinfo))
        .collect::<Vec<_>>();
    let retval = guest.entry(&mut store, args.as_slice())??;

    use crate::guest::ValueResult;
    Ok(match retval {
        Some(ValueResult::String(v)) => v.into_datum().unwrap(),
        Some(ValueResult::I32(v)) => v.into_datum().unwrap(),
        Some(ValueResult::I64(v)) => v.into_datum().unwrap(),
        Some(ValueResult::Bool(v)) => v.into_datum().unwrap(),
        None => Option::<()>::None.into_datum().unwrap(),
    })
}

pub(crate) unsafe fn unload_function(fn_oid: pg_sys::Oid) {
    CACHE.remove(&fn_oid);
}

pub(crate) fn compile_function(fn_oid: pg_sys::Oid) -> eyre::Result<(PathBuf, String)> {
    let work_dir = gucs::work_dir();

    let (crate_name, crate_dir) = crate_name_and_path(fn_oid);

    std::fs::create_dir_all(&crate_dir)
        .map_err(|e| PlRustError::ModuleFileGeneration(crate_dir.clone(), e))?;

    let source_code = create_function_crate(fn_oid, &crate_dir, &crate_name)?;

    let wasm_build_output = Command::new("cargo")
        .current_dir(&crate_dir)
        .arg("build")
        .arg("--target")
        .arg("wasm32-wasi")
        .arg("--release")
        .arg("--message-format=json-render-diagnostics")
        .output()
        .map_err(|e| PlRustError::ModuleBuildExecution(e))?;

    let wasm_build_command_bytes = wasm_build_output.stdout;
    let wasm_build_command_reader = BufReader::new(wasm_build_command_bytes.as_slice());
    let wasm_build_command_stream =
        cargo_metadata::Message::parse_stream(wasm_build_command_reader);
    let wasm_build_command_messages = wasm_build_command_stream
        .collect::<Result<Vec<_>, std::io::Error>>()
        .map_err(|e| PlRustError::CargoMessageParse(e))?;

    let build_output_stderr = String::from_utf8(wasm_build_output.stderr)?;

    let result = if !wasm_build_output.status.success() {
        Err(
            eyre!(PlRustError::ModuleExitNonZero(wasm_build_output.status))
                .with_section(move || source_code.header("Source Code:"))
                .with_section(move || build_output_stderr.header("Stderr:")),
        )
    } else {
        let mut wasm_module = None;
        for message in wasm_build_command_messages {
            match message {
                cargo_metadata::Message::CompilerArtifact(artifact) => {
                    if artifact.target.name != *crate_name {
                        continue;
                    }
                    for filename in &artifact.filenames {
                        if filename.extension() == Some("wasm") {
                            wasm_module = Some(filename.to_string());
                            break;
                        }
                    }
                }
                cargo_metadata::Message::CompilerMessage(_)
                | cargo_metadata::Message::BuildScriptExecuted(_)
                | cargo_metadata::Message::BuildFinished(_)
                | _ => (),
            }
        }

        match wasm_module {
            Some(wasm_module) => {
                let mut final_path = work_dir.clone();
                final_path.push(&format!("{}.wasm", crate_name));

                // move the wasm module into its final location, which is
                // at the root of the configured `work_dir`
                std::fs::rename(&wasm_module, &final_path)
                    .map_err(|e| PlRustError::ModuleRelocation(e))?;

                Ok((final_path, build_output_stderr))
            }
            None => Err(eyre!(PlRustError::ModuleNotFound(crate_name))),
        }
    };

    std::fs::remove_dir_all(&crate_dir).map_err(|e| PlRustError::Cleanup(crate_dir.into(), e))?;

    result
}

fn create_function_crate(
    fn_oid: pg_sys::Oid,
    crate_dir: &PathBuf,
    crate_name: &str,
) -> eyre::Result<String> {
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
wit-bindgen-rust = {{ git = "https://github.com/bytecodealliance/wit-bindgen.git", rev = "bb33644b4fd21ecf43761f63c472cdfffe57e300" }}
{dependencies}
"#,
            crate_name = crate_name,
            dependencies = dependencies,
        ),
    )
    .expect("failed to write Cargo.toml");

    // the src/ directory
    let mut src = crate_dir.clone();
    src.push("src");
    std::fs::create_dir_all(&src).expect("failed to create src directory");

    let mut toolbox_rs = src.clone();
    toolbox_rs.push("toolbox.rs");
    std::fs::write(toolbox_rs, include_str!("../guest_src/toolbox.rs")).unwrap();

    // the actual source code in src/lib.rs
    let mut lib_rs = src.clone();
    lib_rs.push("lib.rs");

    let source_code_formatted = prettyplease::unparse(&source_code);
    std::fs::write(&lib_rs, &source_code_formatted)
        .map_err(|e| PlRustError::ModuleFileGeneration(lib_rs.into(), e))?;

    Ok(source_code_formatted)
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
        let arg_ty = oid_to_syn_type(arg_type_oid, true).unwrap();
        let arg_name = match arg_name {
            Some(name) if name.len() > 0 => name.clone(),
            _ => format!("arg{}", arg_idx),
        };
        let arg_ident: syn::Ident = syn::parse_str(&arg_name).expect("Invalid ident");

        user_fn_arg_idents.push(arg_ident);
        user_fn_arg_types.push(arg_ty);
    }
    let user_fn_block_tokens: syn::Block =
        syn::parse_str(&format!("{{ {} }}", code)).expect("Couldn't parse user code");
    let user_fn_return_tokens = oid_to_syn_type(return_type, true);

    let user_fn_tokens: syn::ItemFn = syn::parse_quote! {
        fn #user_fn_ident(
            #( #user_fn_arg_idents: #user_fn_arg_types ),*
        ) -> Result<Option<#user_fn_return_tokens>, guest::Error>
        #user_fn_block_tokens
    };
    source.items.push(syn::Item::Fn(user_fn_tokens));

    let mut entry_fn_arg_transform_tokens: Vec<syn::Expr> = Vec::default();
    for (_arg_type_oid, _arg_name) in args.iter() {
        entry_fn_arg_transform_tokens.push(syn::parse_quote! { args.pop().unwrap().try_into()? });
    }

    let guest_wit_path = WORK_DIR_GUEST_WIT
        .canonicalize()
        .unwrap()
        .display()
        .to_string();
    let host_wit_path = WORK_DIR_HOST_WIT
        .canonicalize()
        .unwrap()
        .display()
        .to_string();

    source.items.push(syn::parse_quote! {
        // Various implementations to support bindings.
        mod toolbox;
    });

    source.items.push(syn::parse_quote! {
        wit_bindgen_rust::import!(#host_wit_path);
    });
    source.items.push(syn::parse_quote! {
        wit_bindgen_rust::export!(#guest_wit_path);
    });
    source.items.push(syn::parse_quote! {
        struct Guest;
    });
    source.items.push(syn::parse_quote! {
        impl guest::Guest for Guest {
            #[allow(unused_variables, unused_mut)] // In case of zero args.
            fn entry(
                mut args: Vec<guest::Value>
            ) -> Result<Option<guest::Value>, guest::Error> {
                let retval = #user_fn_ident(
                    #(#entry_fn_arg_transform_tokens),*
                )?;
                Ok(retval.map(|v| v.into()))
            }
        }
    });

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

fn oid_to_syn_type(type_oid: &PgOid, owned: bool) -> Option<syn::Type> {
    let array_type = unsafe { pg_sys::get_element_type(type_oid.value()) };

    let (base_oid, array) = if array_type != pg_sys::InvalidOid {
        (PgOid::from(array_type), true)
    } else {
        (type_oid.clone(), false)
    };

    let base_rust_type: syn::Type = match base_oid {
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
