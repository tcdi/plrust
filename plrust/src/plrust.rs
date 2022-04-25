/*
Portions Copyright 2020-2021 ZomboDB, LLC.
Portions Copyright 2021-2022 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

use crate::gucs;
use pgx::pg_sys::heap_tuple_get_struct;
use pgx::*;
use std::{cell::RefCell, collections::BTreeMap, io::BufReader, path::PathBuf, process::Command};

use crate::{error::PlRustError, wasm_executor::WasmExecutor};
use color_eyre::{Section, SectionExt};
use eyre::eyre;
use include_dir::include_dir;
use quote::quote;

static GUEST_TEMPLATE_DIR: include_dir::Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/../guest_template");
static GUEST_INTERFACE_DIR: include_dir::Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/../components/interface");
static WIT_DIR: include_dir::Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../components/wit");

// We use a thread local to avoid having to do any locking or atomics.
// Postgres (and thus PL/Rust) generally run single threaded.
thread_local! {
    static EXECUTOR: RefCell<WasmExecutor> = RefCell::new(WasmExecutor::new().expect("Could not instantiate WasmExecutor"));
}

fn interface_dir() -> PathBuf {
    let mut path = gucs::work_dir().clone();
    path.push("components");
    path.push("interface");
    path
}

fn wit_dir() -> PathBuf {
    let mut path = gucs::work_dir().clone();
    path.push("components");
    path.push("wit");
    path
}

pub(crate) fn init() {
    let interface_dir = interface_dir();
    // std::fs::remove_dir_all(&interface_dir).ok();
    std::fs::create_dir_all(&interface_dir).expect("Could not initialize interface crate");
    GUEST_INTERFACE_DIR
        .extract(&interface_dir)
        .expect("Could not extract Guest interface crate");

    let wit_dir = wit_dir();
    // std::fs::remove_dir_all(&wit_dir).ok();
    std::fs::create_dir_all(&wit_dir).expect("Could not initialize wit directory");
    WIT_DIR
        .extract(&wit_dir)
        .expect("Could not extract WIT definitions");
}

/// Executes the wasm related to a given `fn_oid`.
///
/// If this instance of the extension hasn't yet instantiated it, do that first.
pub(crate) fn execute(
    fn_oid: &pg_sys::Oid,
    fcinfo: &pg_sys::FunctionCallInfo,
) -> eyre::Result<pg_sys::Datum> {
    EXECUTOR.with(|executor| {
        let mut executor = executor.try_borrow_mut()?;

        let guest = match executor.guest(&fn_oid) {
            Some(guest) => guest,
            None => executor.instantiate(*fn_oid)?,
        };

        guest.entry(&fcinfo)
    })
}

// Unloads the wasm guest for a given `fn_oid`.
pub(crate) fn unload(fn_oid: &pg_sys::Oid) -> eyre::Result<()> {
    EXECUTOR.with(|executor| {
        let mut executor = executor.try_borrow_mut()?;

        let _ = executor.remove(fn_oid);
        Ok(())
    })
}

/// Compiles the wasm related to a given `fn_oid` and retains the produced artifact in the `gucs::work_dir`.
pub(crate) fn compile(fn_oid: pg_sys::Oid) -> eyre::Result<PathBuf> {
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

                Ok(final_path)
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
    let (fn_oid, mut dependencies, code, args, (return_type, is_set), is_strict) =
        extract_code_and_args(fn_oid);
    let mut source_code =
        generate_function_source(fn_oid, &code, &args, &return_type, is_set, is_strict)?;

    GUEST_TEMPLATE_DIR
        .extract(crate_dir)
        .expect("Could not extract Guest template");

    // Update cargo toml
    let cargo_toml_path = {
        let mut file = crate_dir.clone();
        file.push("Cargo.toml");
        file
    };
    let cargo_toml_string = std::fs::read_to_string(&cargo_toml_path).unwrap();
    let mut updated_cargo_toml = ::cargo_toml::Manifest::from_str(&cargo_toml_string)?;
    if let Some(ref mut package) = updated_cargo_toml.package {
        package.name = crate_name.to_string();
    }
    // Patch interface path
    let _old_interface_dep = updated_cargo_toml.dependencies.insert(
        "interface".to_string(),
        cargo_toml::Dependency::Detailed(cargo_toml::DependencyDetail {
            path: Some(interface_dir().display().to_string()),
            ..Default::default()
        }),
    );
    updated_cargo_toml.dependencies.append(&mut dependencies);

    // TODO: Include user deps

    let updated_cargo_toml_string = toml::to_string(&updated_cargo_toml).unwrap();
    std::fs::write(&cargo_toml_path, &updated_cargo_toml_string)
        .map_err(|e| PlRustError::ModuleFileGeneration(cargo_toml_path.into(), e))?;

    let lib_rs_path = {
        let mut filename = crate_dir.clone();
        filename.push("src");
        filename.push("lib.rs");
        filename
    };
    let lib_rs_source = std::fs::read_to_string(&lib_rs_path).unwrap();
    let mut lib_rs = syn::parse_file(&lib_rs_source).unwrap();
    // The last item is `mod smoke_test {}` (TODO: Assert)
    lib_rs.items.remove(lib_rs.items.len() - 1);
    lib_rs.items.append(&mut source_code);

    let lib_rs_formatted = prettyplease::unparse(&lib_rs);
    std::fs::write(&lib_rs_path, &lib_rs_formatted)
        .map_err(|e| PlRustError::ModuleFileGeneration(lib_rs_path.into(), e))?;

    Ok(lib_rs_formatted)
}

fn crate_name(fn_oid: pg_sys::Oid) -> String {
    let db_oid = unsafe { pg_sys::MyDatabaseId };
    let ns_oid = unsafe { pg_sys::get_func_namespace(fn_oid) };
    format!("fn{}_{}_{}", db_oid, ns_oid, fn_oid)
}

pub(crate) fn crate_name_and_path(fn_oid: pg_sys::Oid) -> (String, PathBuf) {
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
) -> eyre::Result<Vec<syn::Item>> {
    let mut items = Vec::new();

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
        user_fn_arg_types.push(syn::parse2(quote! { Option<#arg_ty> })?);
    }
    let user_fn_block_tokens: syn::Block =
        syn::parse_str(&format!("{{ {} }}", code)).expect("Couldn't parse user code");
    let user_fn_return_tokens = oid_to_syn_type(return_type, true);
    let user_fn_arg_idents_len = user_fn_arg_idents.len() as u64;

    let user_fn_tokens: syn::ItemFn = syn::parse2(quote! {
        fn #user_fn_ident(
            #( #user_fn_arg_idents: #user_fn_arg_types ),*
        ) -> Result<Option<#user_fn_return_tokens>, guest::Error>
        #user_fn_block_tokens
    })?;
    items.push(syn::Item::Fn(user_fn_tokens));

    let mut entry_fn_arg_transform_tokens: Vec<syn::Expr> = Vec::default();
    for ident in user_fn_arg_idents.iter() {
        entry_fn_arg_transform_tokens
            .push(syn::parse2(quote! { #ident.map(|v| v.try_into()).transpose()? })?);
    }

    let entry_fn = quote! {
        impl guest::Guest for Guest {
            #[allow(unused_variables, unused_mut)] // In case of zero args.
            fn entry(
                mut args: Vec<Option<guest::Value>>
            ) -> Result<Option<guest::Value>, guest::Error> {
                let args_len = args.len() as u64;
                let [ #(#user_fn_arg_idents),* ]: [_; #user_fn_arg_idents_len as usize] = args.try_into()
                    .map_err(|_e| guest::Error::mismatched_args_length(#user_fn_arg_idents_len, args_len))?;
                let retval = #user_fn_ident(
                    #(#entry_fn_arg_transform_tokens),*
                )?;
                Ok(retval.map(|v| v.into()))
            }
        }
    };
    items.push(
        syn::parse2(entry_fn.clone())
            .map_err(|e| eyre!(e).with_section(|| entry_fn.to_string().header("Source code:")))?
    );

    Ok(items)
}

fn extract_code_and_args(
    fn_oid: pg_sys::Oid,
) -> (
    pg_sys::Oid,
    BTreeMap<std::string::String, cargo_toml::Dependency>,
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
        let deps_parsed: BTreeMap<std::string::String, cargo_toml::Dependency> =
            toml::from_str(&deps).unwrap();
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

        (
            fn_oid,
            deps_parsed,
            source_code,
            args,
            return_type,
            is_strict,
        )
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
