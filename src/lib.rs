/*
Portions Copyright 2020-2021 ZomboDB, LLC.
Portions Copyright 2021-2022 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

#![doc = include_str!("../README.md")]

mod error;
mod gucs;
mod logging;
mod plrust;

use error::PlRustError;
use pgx::*;
use std::error::Error;

pg_module_magic!();

#[pg_guard]
fn _PG_init() {
    color_eyre::config::HookBuilder::default()
        .theme(color_eyre::config::Theme::new())
        .into_hooks()
        .1
        .install()
        .unwrap();

    gucs::init();

    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

    let filter_layer = match EnvFilter::try_from_default_env() {
        Ok(layer) => layer,
        Err(e) => {
            // Catch a parse error and report it, ignore a missing env.
            if let Some(source) = e.source() {
                match source.downcast_ref::<std::env::VarError>() {
                    Some(std::env::VarError::NotPresent) => (),
                    Some(e) => panic!("Error parsing RUST_LOG directives: {}", e),
                    None => panic!("Error parsing RUST_LOG directives"),
                }
            }
            EnvFilter::try_new(&format!("{}=info", env!("CARGO_PKG_NAME")))
                .expect("Error parsing default log level")
        }
    }
    .add_directive(gucs::tracing_level().into());

    let error_layer = tracing_error::ErrorLayer::default();

    let format_layer = tracing_subscriber::fmt::Layer::new()
        .with_ansi(false)
        .with_writer(|| logging::PgxNoticeWriter::<true>)
        .without_time()
        .pretty();
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(format_layer)
        .with(error_layer)
        .try_init()
        .expect("Could not initialize tracing registry");

    plrust::init();
}

/// `pgx` doesn't know how to declare a CREATE FUNCTION statement for a function
/// whose only argument is a `pg_sys::FunctionCallInfo`, so we gotta do that ourselves.
#[pg_extern(sql = "
CREATE FUNCTION plrust_call_handler() RETURNS language_handler
    LANGUAGE c AS 'MODULE_PATHNAME', '@FUNCTION_NAME@';
")]
#[tracing::instrument(level = "debug")]
unsafe fn plrust_call_handler(fcinfo: pg_sys::FunctionCallInfo) -> pg_sys::Datum {
    unsafe fn plrust_call_handler_inner(
        fcinfo: pg_sys::FunctionCallInfo,
    ) -> eyre::Result<pg_sys::Datum> {
        let fn_oid = fcinfo
            .as_ref()
            .ok_or(PlRustError::NullFunctionCallInfo)?
            .flinfo
            .as_ref()
            .ok_or(PlRustError::NullFmgrInfo)?
            .fn_oid;
        let func = plrust::lookup_function(fn_oid)?;

        Ok(func(fcinfo))
    }

    match plrust_call_handler_inner(fcinfo) {
        Ok(datum) => datum,
        // Panic into the pgx guard.
        Err(err) => panic!("{:?}", err),
    }
}

#[pg_extern]
#[tracing::instrument(level = "debug")]
unsafe fn plrust_validator(fn_oid: pg_sys::Oid, fcinfo: pg_sys::FunctionCallInfo) {
    unsafe fn plrust_validator_inner(
        fn_oid: pg_sys::Oid,
        fcinfo: pg_sys::FunctionCallInfo,
    ) -> eyre::Result<()> {
        let fcinfo = PgBox::from_pg(fcinfo);
        let flinfo = PgBox::from_pg(fcinfo.flinfo);
        if !pg_sys::CheckFunctionValidatorAccess(
            flinfo.fn_oid,
            pg_getarg(fcinfo.as_ptr(), 0).unwrap(),
        ) {
            return Err(PlRustError::CheckFunctionValidatorAccess)?;
        }

        plrust::unload_function(fn_oid);
        // NOTE:  We purposely ignore the `check_function_bodies` GUC for compilation as we need to
        // compile the function when it's created to avoid locking during function execution
        let (_, _, stderr) = plrust::compile_function(fn_oid)?;

        // however, we'll use it to decide if we should go ahead and dynamically load our function
        if pg_sys::check_function_bodies {
            // it's on, so lets go ahead and load our function
            // plrust::lookup_function(fn_oid);
        }

        // if the compilation had warnings we'll display them
        if stderr.contains("warning: ") {
            pgx::warning!("\n{}", stderr);
        }

        Ok(())
    }

    match plrust_validator_inner(fn_oid, fcinfo) {
        Ok(()) => (),
        // Panic into the pgx guard.
        Err(err) => panic!("{:?}", err),
    }
}

#[pg_extern]
#[tracing::instrument(level = "debug")]
fn recompile_function(
    fn_oid: pg_sys::Oid,
) -> (
    name!(library_path, Option<String>),
    name!(stdout, Option<String>),
    name!(stderr, Option<String>),
    name!(plrust_error, Option<String>),
) {
    unsafe {
        plrust::unload_function(fn_oid);
    }
    match plrust::compile_function(fn_oid) {
        Ok((work_dir, stdout, stderr)) => (
            Some(work_dir.display().to_string()),
            Some(stdout),
            Some(stderr),
            None,
        ),
        Err(err) => (None, None, None, Some(format!("{:?}", err))),
    }
}

extension_sql!(
    r#"
CREATE LANGUAGE plrust
    HANDLER plrust.plrust_call_handler
    VALIDATOR plrust.plrust_validator;

COMMENT ON LANGUAGE plrust IS 'PL/rust procedural language';
"#,
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
            CREATE FUNCTION sum_array(a BIGINT[]) RETURNS BIGINT
                IMMUTABLE STRICT
                LANGUAGE PLRUST AS
            $$
                Some(a.into_iter().map(|v| v.unwrap_or_default()).sum())
            $$;
        "#;
        Spi::run(definition);

        let retval = Spi::get_one_with_args(
            r#"
            SELECT sum_array($1);
        "#,
            vec![(
                PgBuiltInOids::INT4ARRAYOID.oid(),
                vec![1, 2, 3].into_datum(),
            )],
        );
        assert_eq!(retval, Some(6));
    }

    #[pg_test]
    #[search_path(@extschema@)]
    fn test_update() {
        let definition = r#"
            CREATE FUNCTION update_me() RETURNS TEXT
                IMMUTABLE STRICT
                LANGUAGE PLRUST AS
            $$
                String::from("booper").into()
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
                String::from("swooper").into()
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
            CREATE FUNCTION random_contributor_pet() RETURNS TEXT
                STRICT
                LANGUAGE PLRUST AS
            $$
                let name = Spi::get_one("SELECT name FROM contributors_pets ORDER BY random() LIMIT 1");
                name
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
            CREATE FUNCTION contributor_pet(name TEXT) RETURNS INT
                STRICT
                LANGUAGE PLRUST AS
            $$
                let id = Spi::get_one_with_args(
                    "SELECT id FROM contributors_pets WHERE name = $1",
                    vec![(PgBuiltInOids::TEXTOID.oid(), name.into_datum())],
                );
                id
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
            CREATE FUNCTION zalgo(input TEXT) RETURNS TEXT
                IMMUTABLE STRICT
                LANGUAGE PLRUST AS
            $$
            [dependencies]
                zalgo = "0.2.0"
            [code]
                use zalgo::{Generator, GeneratorArgs, ZalgoSize};

                let mut generator = Generator::new();
                let mut out = String::new();
                let args = GeneratorArgs::new(true, false, false, ZalgoSize::Maxi);
                let _result = generator.gen(input, &mut out, &args);

                Some(out)
            $$;
        "#;
        Spi::run(definition);

        let retval: Option<String> = Spi::get_one_with_args(
            r#"
            SELECT zalgo($1);
        "#,
            vec![(PgBuiltInOids::TEXTOID.oid(), "Nami".into_datum())],
        );
        assert!(retval.is_some());
    }
}

#[cfg(test)]
pub mod pg_test {
    use once_cell::sync::Lazy;
    use pgx::utils::pg_config::Pgx;
    use tempdir::TempDir;

    static WORK_DIR: Lazy<String> = Lazy::new(|| {
        let work_dir = TempDir::new("plrust-tests").expect("Couldn't create tempdir");
        format!("plrust.work_dir='{}'", work_dir.path().display())
    });
    static PG_CONFIG: Lazy<String> = Lazy::new(|| {
        let pgx_config = Pgx::from_config().unwrap();
        let version = format!("pg{}", pgx::pg_sys::get_pg_major_version_num());
        let pg_config = pgx_config.get(&version).unwrap();
        let path = pg_config.path().unwrap();
        format!("plrust.pg_config='{}'", path.as_path().display())
    });

    pub fn setup(_options: Vec<&str>) {
        // perform one-off initialization when the pg_test framework starts
    }

    pub fn postgresql_conf_options() -> Vec<&'static str> {
        vec![&*WORK_DIR, &*PG_CONFIG]
    }
}
