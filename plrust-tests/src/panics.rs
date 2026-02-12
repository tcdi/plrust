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
    #[search_path(@extschema@)]
    #[should_panic = "yup"]
    fn pgrx_can_panic() {
        panic!("yup")
    }

    #[pg_test]
    #[search_path(@extschema@)]
    #[should_panic = "yup"]
    fn plrust_can_panic() -> spi::Result<()> {
        let definition = r#"
            CREATE FUNCTION shut_up_and_explode()
            RETURNS text AS
            $$
                panic!("yup");
                Ok(None)
            $$ LANGUAGE plrust;
        "#;

        Spi::run(definition)?;
        let retval = Spi::get_one::<String>("SELECT shut_up_and_explode();\n");
        assert_eq!(retval, Ok(None));
        Ok(())
    }
    #[pg_test]
    #[search_path(@extschema@)]
    #[should_panic = "xxx"]
    #[ignore]
    fn plrust_pgloglevel_dont_allcaps_panic() -> spi::Result<()> {
        // This test attempts to annihilate the database.
        // It relies on the existing assumption that tests are run in the same Postgres instance,
        // so this test will make all tests "flaky" if Postgres suddenly goes down with it.
        let definition = r#"
            CREATE FUNCTION dont_allcaps_panic()
            RETURNS text AS
            $$
                ereport!(PANIC, PgSqlErrorCode::ERRCODE_INTERNAL_ERROR, "If other tests completed, PL/Rust did not actually destroy the entire database, \
                                         But if you see this in the error output, something might be wrong.");
                Ok(Some("lol".into()))
            $$ LANGUAGE plrust;
        "#;
        Spi::run(definition)?;
        let retval = Spi::get_one::<String>("SELECT dont_allcaps_panic();\n");
        assert_eq!(retval, Ok(Some("lol".into())));
        Ok(())
    }
}
