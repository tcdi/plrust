/*
Portions Copyright 2020-2021 ZomboDB, LLC.
Portions Copyright 2021-2022 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

mod error;
mod gucs;
pub mod interface;
mod plrust;
mod logging;

use error::PlRustError;
use pgx::*;

wit_bindgen_wasmtime::export!("../components/wit/host.wit");
wit_bindgen_wasmtime::import!("../components/wit/guest.wit");

pg_module_magic!();

#[pg_guard]
fn _PG_init() {
    color_eyre::config::HookBuilder::default()
        .theme(if !atty::is(atty::Stream::Stderr) {
            color_eyre::config::Theme::new()
        } else {
            color_eyre::config::Theme::default()
        })
        .into_hooks()
        .1
        .install()
        .unwrap();

    gucs::init();
    plrust::init();
}

/// `pgx` doesn't know how to declare a CREATE FUNCTION statement for a function
/// whose only argument is a `pg_sys::FunctionCallInfo`, so we gotta do that ourselves.
#[pg_extern(sql = "\
CREATE OR REPLACE FUNCTION plrust_call_handler() RETURNS language_handler
    LANGUAGE c AS 'MODULE_PATHNAME', '@FUNCTION_NAME@';\
")]
unsafe fn plrust_call_handler(fcinfo: pg_sys::FunctionCallInfo) -> pg_sys::Datum {
    match plrust_call_handler_inner(fcinfo) {
        Ok(datum) => datum,
        // Panic into the pgx guard.
        Err(e) => panic!("{:?}", e),
    }
}

unsafe fn plrust_call_handler_inner(
    fcinfo: pg_sys::FunctionCallInfo,
) -> eyre::Result<pg_sys::Datum> {
    let fn_oid = fcinfo
        .as_ref()
        .ok_or(PlRustError::FunctionCallInfoWasNone)?
        .flinfo
        .as_ref()
        .ok_or(PlRustError::FnOidWasNone)?
        .fn_oid;
    plrust::execute_wasm_function(fn_oid, fcinfo)
}

#[pg_extern]
unsafe fn plrust_validator(fn_oid: pg_sys::Oid, fcinfo: pg_sys::FunctionCallInfo) {
    match plrust_validator_inner(fn_oid, fcinfo) {
        Ok(()) => (),
        // Panic into the pgx guard.
        Err(e) => panic!("{:?}", e),
    }
}

unsafe fn plrust_validator_inner(
    fn_oid: pg_sys::Oid,
    fcinfo: pg_sys::FunctionCallInfo,
) -> eyre::Result<()> {
    let fcinfo = PgBox::from_pg(fcinfo);
    let flinfo = PgBox::from_pg(fcinfo.flinfo);
    if !pg_sys::CheckFunctionValidatorAccess(
        flinfo.fn_oid,
        pg_getarg(fcinfo.as_ptr(), 0).ok_or(PlRustError::PgGetArgWasNone(fn_oid, 0))?,
    ) {
        return Ok(());
    }

    plrust::unload_function(fn_oid);
    // NOTE:  We purposely ignore the `check_function_bodies` GUC for compilation as we need to
    // compile the function when it's created to avoid locking during function execution
    let (_, output) = plrust::compile_function(fn_oid)?;

    // if the compilation had warnings we'll display them
    if output.contains("warning: ") {
        pgx::warning!("\n{}", output);
    }
    Ok(())
}

#[pg_extern]
fn recompile_function(
    fn_oid: pg_sys::Oid,
) -> (
    name!(library_path, Option<String>),
    name!(cargo_output, String),
) {
    unsafe {
        plrust::unload_function(fn_oid);
    }
    match plrust::compile_function(fn_oid) {
        Ok((work_dir, output)) => (Some(work_dir.display().to_string()), output),
        Err(e) => (None, format!("{}", e)),
    }
}

extension_sql!(
    "\
CREATE LANGUAGE plrust
    HANDLER plrust.plrust_call_handler
    VALIDATOR plrust.plrust_validator;
    
COMMENT ON LANGUAGE plrust IS 'PL/rust procedural language';\
",
    name = "language_handler",
    requires = [plrust_call_handler, plrust_validator]
);

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use super::*;

    // Bootstrap a testing table for non-immutable functions
    extension_sql!(
        r#"   
        CREATE TABLE contributors_pets (
            id serial8 not null primary key,
            name text
        );
        INSERT INTO contributors_pets (name) VALUES ('Brandy');
        INSERT INTO contributors_pets (name) VALUES ('Nami');
        INSERT INTO contributors_pets (name) VALUES ('Sally');
        INSERT INTO contributors_pets (name) VALUES ('Anchovy');
    "#,
        name = "create_contributors_pets",
    );

    #[pg_test]
    #[search_path(@extschema@)]
    fn test_basic() {
        let definition = r#"
            CREATE OR REPLACE FUNCTION update_me() RETURNS TEXT
                IMMUTABLE STRICT
                LANGUAGE PLRUST AS
            $$
                Ok(Some(String::from("booper")))
            $$;
        "#;
        Spi::run(definition);

        let retval = Spi::get_one(
            r#"
            SELECT update_me();
        "#,
        );
        assert_eq!(retval, Some("booper"));
    }

    // #[pg_test]
    // #[search_path(@extschema@)]
    // fn test_lists() {
    //     let definition = r#"
    //         CREATE OR REPLACE FUNCTION sum_array(a BIGINT[]) RETURNS BIGINT
    //             IMMUTABLE STRICT
    //             LANGUAGE PLRUST AS
    //         $$
    //             Ok(a.into_iter().map(|v| v.unwrap_or_default()).sum())
    //         $$;
    //     "#;
    //     Spi::run(definition);

    //     let retval = Spi::get_one_with_args(
    //         r#"
    //         SELECT sum_array($1);
    //     "#,
    //         vec![(
    //             PgBuiltInOids::INT4ARRAYOID.oid(),
    //             vec![1, 2, 3].into_datum(),
    //         )],
    //     );
    //     assert_eq!(retval, Some(6));
    // }

    #[pg_test]
    #[search_path(@extschema@)]
    fn test_update() {
        let definition = r#"
            CREATE OR REPLACE FUNCTION update_me() RETURNS TEXT
                IMMUTABLE STRICT
                LANGUAGE PLRUST AS
            $$
                Ok(Some(String::from("booper")))
            $$;
        "#;
        Spi::run(definition);

        let retval = Spi::get_one(
            r#"
            SELECT update_me();
        "#,
        );
        assert_eq!(retval, Some("booper"));

        let definition = r#"
            CREATE OR REPLACE FUNCTION update_me() RETURNS TEXT
                IMMUTABLE STRICT
                LANGUAGE PLRUST AS
            $$

                Ok(Some(String::from("swooper")))
            $$;
        "#;
        Spi::run(definition);

        let retval = Spi::get_one(
            r#"
            SELECT update_me();
        "#,
        );
        assert_eq!(retval, Some("swooper"));
    }

    #[pg_test]
    #[search_path(@extschema@)]
    fn test_spi() {
        let random_definition = r#"
            CREATE OR REPLACE FUNCTION random_contributor_pet() RETURNS TEXT
                STRICT
                LANGUAGE PLRUST AS
            $$
                let name: Option<String> = interface::get_one(
                    "SELECT name FROM contributors_pets ORDER BY random() LIMIT 1",
                    interface::ValueType::String,
                ).map_err(Into::into).transpose().map(|v| v.and_then(|i| i.try_into())).transpose()?;
                Ok(name)
            $$;
        "#;
        Spi::run(random_definition);

        let retval: Option<String> = Spi::get_one(
            r#"
            SELECT random_contributor_pet();
        "#,
        );
        assert!(retval.is_some());

        let specific_definition = r#"
            CREATE OR REPLACE FUNCTION contributor_pet(name TEXT) RETURNS INT
                STRICT
                LANGUAGE PLRUST AS
            $$
                let id: Option<i32> = interface::get_one_with_args(
                    "SELECT id FROM contributors_pets WHERE name = $1",
                    &[name.unwrap().as_str().into()],
                    interface::ValueType::I32,
                ).map_err(Into::into).transpose().map(|v| v.and_then(|i| i.try_into())).transpose()?;

                Ok(id)
            $$;
        "#;
        Spi::run(specific_definition);

        let retval: Option<i32> = Spi::get_one(
            r#"
            SELECT contributor_pet('Nami');
        "#,
        );
        assert_eq!(retval, Some(2));
    }

    #[pg_test]
    #[cfg(not(feature = "sandboxed"))]
    #[search_path(@extschema@)]
    fn test_deps() {
        let definition = r#"
            CREATE OR REPLACE FUNCTION colorize(input TEXT) RETURNS TEXT
                IMMUTABLE STRICT
                LANGUAGE PLRUST AS
            $$
            [dependencies]
                owo-colors = "3"
            [code]
                use owo_colors::OwoColorize;

                Ok(Some(input.unwrap().purple().to_string()))
            $$;
        "#;
        Spi::run(definition);

        let retval: Option<String> = Spi::get_one_with_args(
            r#"
            SELECT colorize($1);
        "#,
            vec![(PgBuiltInOids::TEXTOID.oid(), "Nami".into_datum())],
        );
        assert!(retval.is_some());
    }
}

#[cfg(test)]
pub mod pg_test {
    use once_cell::sync::Lazy;
    use tempdir::TempDir;

    static WORK_DIR: Lazy<String> = Lazy::new(|| {
        let work_dir = TempDir::new("plrust-tests").expect("Couldn't create tempdir");
        format!("plrust.work_dir='{}'", work_dir.path().display())
    });

    pub fn setup(_options: Vec<&str>) {
        // perform one-off initialization when the pg_test framework starts
    }

    pub fn postgresql_conf_options() -> Vec<&'static str> {
        vec![&*WORK_DIR]
    }
}
