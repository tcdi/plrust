// Copyright (c) 2020, ZomboDB, LLC
use crate::gucs;
use libloading::{Library, Symbol};
use pgx::pg_sys::heap_tuple_get_struct;
use pgx::*;
use std::{collections::HashMap, path::PathBuf, process::Command};

static mut LOADED_SYMBOLS: Option<
    HashMap<
        pg_sys::Oid,
        (
            Library,
            Option<
                Symbol<'static, unsafe extern "C" fn(pg_sys::FunctionCallInfo) -> pg_sys::Datum>,
            >,
        ),
    >,
> = None;

pub(crate) fn init() {
    unsafe {
        LOADED_SYMBOLS = Some(HashMap::new());
    }
}

pub(crate) unsafe fn unload_function(fn_oid: pg_sys::Oid) {
    LOADED_SYMBOLS.as_mut().unwrap().remove(&fn_oid);
}

pub(crate) unsafe fn lookup_function(
    fn_oid: pg_sys::Oid,
) -> &'static Symbol<'static, unsafe extern "C" fn(pg_sys::FunctionCallInfo) -> pg_sys::Datum> {
    let (library, symbol) = LOADED_SYMBOLS
        .as_mut()
        .unwrap()
        .entry(fn_oid)
        .or_insert_with(|| {
            let mut shared_library = gucs::work_dir();
            let crate_name = crate_name(fn_oid);

            shared_library.push(&format!("{}.so", crate_name));
            let library = Library::new(&shared_library).unwrap_or_else(|e| {
                panic!(
                    "failed to open shared library at `{}`: {}",
                    shared_library.display(),
                    e
                )
            });

            (library, None)
        });

    match symbol {
        Some(symbol) => symbol,
        None => {
            let symbol_name = format!("plrust_fn_{}_wrapper", fn_oid);
            symbol.replace(
                library
                    .get(&symbol_name.as_bytes())
                    .expect("failed to find function"),
            );
            symbol.as_ref().unwrap()
        }
    }
}

pub(crate) fn compile_function(fn_oid: pg_sys::Oid) -> Result<(PathBuf, String), String> {
    let work_dir = gucs::work_dir();
    let (crate_name, crate_dir) = crate_name_and_path(fn_oid);

    std::fs::create_dir_all(&crate_dir).expect("failed to create crate directory");

    let source_code = create_function_crate(fn_oid, &crate_dir, &crate_name);

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
        .expect("failed to build function shared library");

    let mut output_string = String::new();
    unsafe {
        output_string.push_str(&String::from_utf8_unchecked(cargo_output.stdout));
        output_string.push_str(&String::from_utf8_unchecked(cargo_output.stderr));
    }

    let result = if !cargo_output.status.success() {
        output_string.push_str("-----------------\n");
        output_string.push_str(&source_code);
        Err(output_string)
    } else {
        match find_shared_library(&crate_name).0 {
            Some(shared_library) => {
                let mut final_path = work_dir.clone();
                final_path.push(&format!("{}.so", crate_name));

                // move the shared_library into its final location, which is
                // at the root of the configured `work_dir`
                std::fs::rename(&shared_library, &final_path)
                    .expect("unable to rename shared_library");

                Ok((final_path, output_string))
            }
            None => Err(output_string),
        }
    };

    // no matter what happened, remove our crate directory, ignoring any error that might generate
    std::fs::remove_dir_all(&crate_dir).ok();

    result
}

fn create_function_crate(fn_oid: pg_sys::Oid, crate_dir: &PathBuf, crate_name: &str) -> String {
    let (fn_oid, deps, code, args, (return_type, is_set), is_strict) =
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

[features]
default = ["pgx/pg{major_version}"]

[dependencies]
pgx = "0.4.0-beta.0"
{dependencies}

[profile.release]
panic = "unwind"
opt-level = 3
lto = "fat"
codegen-units = 1
"#,
            crate_name = crate_name,
            major_version = pg_sys::get_pg_major_version_num(),
            dependencies = deps
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

fn find_shared_library(crate_name: &str) -> (Option<PathBuf>, &str) {
    let work_dir = gucs::work_dir();
    let mut target_dir = work_dir.clone();
    target_dir.push("release");

    // TODO:  we could probably do a conditional compile #[cfg()] thing here

    // linux
    let mut so = target_dir.clone();
    so.push(&format!("lib{}.so", crate_name));
    if so.exists() {
        return (Some(so), crate_name);
    } else {
        // macos
        let mut dylib = target_dir.clone();
        dylib.push(&format!("lib{}.dylib", crate_name));
        if dylib.exists() {
            return (Some(dylib), crate_name);
        } else {
            // windows?
            let mut dll = target_dir.clone();
            dll.push(&format!("lib{}.dll", crate_name));
            if dll.exists() {
                return (Some(dll), crate_name);
            }
        }
    }

    (None, crate_name)
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

    // source header
    source.push_str(
        r#"
use pgx::*;
"#,
    );

    // function name
    source.push_str(&format!(
        r#"
#[pg_extern]
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
        source.push_str(&format!("Option<{}>", make_rust_type(return_type, true)));
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
