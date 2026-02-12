/*
Portions Copyright 2020-2021 ZomboDB, LLC.
Portions Copyright 2021-2025 Technology Concepts & Design, Inc.

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use pgrx::prelude::*;

    #[pg_test]
    #[cfg(not(feature = "sandboxed"))]
    #[search_path(@extschema@)]
    fn plrust_deps_supported_semver_parse() -> spi::Result<()> {
        let definition = r#"
                CREATE FUNCTION colorize(input TEXT) RETURNS TEXT
                IMMUTABLE STRICT
                LANGUAGE PLRUST AS
            $$
            [dependencies]
                owo-colors = ">2"
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
}
