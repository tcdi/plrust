use pgx::*;

mod gucs;
mod plrust;

pg_module_magic!();

#[pg_guard]
fn _PG_init() {
    gucs::init();
    plrust::init();
}

/// `pgx` doesn't know how to declare a CREATE FUNCTION statement for a function
/// whose only argument is a `pg_sys::FunctionCallInfo`, so we gotta do that ourselves.
///
/// ```sql
/// CREATE OR REPLACE FUNCTION plrust_call_handler() RETURNS language_handler
///     LANGUAGE c AS 'MODULE_PATHNAME', 'plrust_call_handler_wrapper';
/// ```
#[pg_extern(raw)]
fn plrust_call_handler(fcinfo: pg_sys::FunctionCallInfo) -> pg_sys::Datum {
    let fn_oid = unsafe { fcinfo.as_ref().unwrap().flinfo.as_ref().unwrap().fn_oid };

    unsafe {
        let func = plrust::lookup_function(fn_oid);
        func(fcinfo)
    }
}

#[pg_extern]
fn plrust_validator(fn_oid: pg_sys::Oid, fcinfo: pg_sys::FunctionCallInfo) {
    let fcinfo = PgBox::from_pg(fcinfo);
    unsafe {
        let flinfo = PgBox::from_pg(fcinfo.flinfo);
        if !pg_sys::CheckFunctionValidatorAccess(
            flinfo.fn_oid,
            pg_getarg(fcinfo.as_ptr(), 0).unwrap(),
        ) {
            return;
        }
    }

    // NOTE:  We purposely ignore the `check_function_bodies` GUC for compilation as we need to
    // compile the function when it's created to avoid locking during function execution
    let (_, output) =
        plrust::compile_function(fn_oid).unwrap_or_else(|e| panic!("compilation failed\n{}", e));

    // however, we'll use it to decide if we should go ahead and dynamically load our function
    unsafe {
        if pg_sys::check_function_bodies {
            // it's on, so lets go ahead and load our function
            plrust::lookup_function(fn_oid);
        }
    }

    // if the compilation had warnings we'll display them
    if output.contains("warning: ") {
        pgx::warning!("\n{}", output);
    }
}

#[pg_extern]
fn recompile_plrust_function(
    fn_oid: pg_sys::Oid,
) -> (
    name!(library_path, Option<String>),
    name!(cargo_output, String),
) {
    match plrust::compile_function(fn_oid) {
        Ok((work_dir, output)) => (Some(work_dir.display().to_string()), output),
        Err(e) => (None, e),
    }
}

extension_sql!(
    r#"
CREATE LANGUAGE plrust
    HANDLER plrust.plrust_call_handler
    VALIDATOR plrust.plrust_validator;
"#
);

#[cfg(any(test, feature = "pg_test"))]
mod tests {}

#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {
        // perform one-off initialization when the pg_test framework starts
    }

    pub fn postgresql_conf_options() -> Vec<&'static str> {
        // return any postgresql.conf settings that are required for your tests
        vec![]
    }
}
