
/*
Portions Copyright 2020-2021 ZomboDB, LLC.
Portions Copyright 2021-2023 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use pgrx:: prelude::*;

    #[pg_test]
    #[cfg(not(feature = "sandboxed"))]
    #[search_path(@extschema@)]
    fn plrust_deps_supported() -> spi::Result<()> {
        let definition = r#"
                CREATE FUNCTION colorize(input TEXT) RETURNS TEXT
                IMMUTABLE STRICT
                LANGUAGE PLRUST AS
            $$
            [dependencies]
                owo-colors = "3"
            [code]
                use owo_colors::OwoColorize;
                Ok(Some(input.purple().to_string()))
            $$;
        "#;
        Spi::run(definition)?;

        let retval = Spi::get_one_with_args::<String>(
            r#"
            SELECT colorize($1);
        "#,
            vec![(PgBuiltInOids::TEXTOID.oid(), "Nami".into_datum())],
        );
        assert!(retval.is_ok());
        assert!(retval.unwrap().is_some());

        // Regression test: A previous version of PL/Rust would abort if this was called twice, so call it twice:
        let retval = Spi::get_one_with_args::<String>(
            r#"
            SELECT colorize($1);
        "#,
            vec![(PgBuiltInOids::TEXTOID.oid(), "Nami".into_datum())],
        );
        assert!(retval.is_ok());
        assert!(retval.unwrap().is_some());
        Ok(())
    }

    #[pg_test]
    #[cfg(not(feature = "sandboxed"))]
    #[search_path(@extschema@)]
    fn plrust_deps_supported_deps_in_toml_table() -> spi::Result<()> {
        let definition = r#"
                CREATE FUNCTION say_hello() RETURNS TEXT
                IMMUTABLE STRICT
                LANGUAGE PLRUST AS
            $$
            [dependencies]
                tokio = ">=1"
                owo-colors = "3"
            [code]
                Ok(Some("hello".to_string()))
            $$;
        "#;
        Spi::run(definition)?;

        let retval = Spi::get_one_with_args::<String>(
            r#"
            SELECT say_hello();
        "#,
            vec![(PgBuiltInOids::TEXTOID.oid(), "hello".into_datum())],
        );
        assert_eq!(retval, Ok(Some("hello".to_string())));
        Ok(())
    }

    #[pg_test]
    #[cfg(not(feature = "sandboxed"))]
    #[search_path(@extschema@)]
    fn plrust_deps_not_supported() {
        let definition = r#"
                CREATE FUNCTION colorize(input TEXT) RETURNS TEXT
                IMMUTABLE STRICT
                LANGUAGE PLRUST AS
            $$
            [dependencies]
                regex = "1.6.5"
            [code]
                Ok(Some("test"))
            $$;
        "#;
        let res = std::panic::catch_unwind(|| {
            Spi::run(definition).expect("SQL for plrust_deps_not_supported() failed")
        });
        assert!(res.is_err());
    }
}