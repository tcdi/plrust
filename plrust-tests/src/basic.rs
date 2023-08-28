/*
Portions Copyright 2020-2021 ZomboDB, LLC.
Portions Copyright 2021-2023 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use pgrx::{datum::IntoDatum, prelude::*};

    #[pg_test]
    #[search_path(@extschema@)]
    fn plrust_basic() -> spi::Result<()> {
        let definition = r#"
            CREATE FUNCTION sum_array(a BIGINT[]) RETURNS BIGINT
                IMMUTABLE STRICT
                LANGUAGE PLRUST AS
            $$
                Ok(Some(a.into_iter().map(|v| v.unwrap_or_default()).sum()))
            $$;
        "#;
        Spi::run(definition)?;

        let retval = Spi::get_one_with_args::<i64>(
            r#"
            SELECT sum_array($1);
        "#,
            vec![(
                PgBuiltInOids::INT4ARRAYOID.oid(),
                vec![1, 2, 3].into_datum(),
            )],
        );
        assert_eq!(retval, Ok(Some(6)));
        Ok(())
    }

    #[pg_test]
    #[search_path(@extschema@)]
    fn plrust_update() -> spi::Result<()> {
        let definition = r#"
            CREATE FUNCTION update_me() RETURNS TEXT
                IMMUTABLE STRICT
                LANGUAGE PLRUST AS
            $$
                Ok(String::from("booper").into())
            $$;
        "#;
        Spi::run(definition)?;

        let retval = Spi::get_one(
            r#"
            SELECT update_me();
        "#,
        );
        assert_eq!(retval, Ok(Some("booper")));

        let definition = r#"
            CREATE OR REPLACE FUNCTION update_me() RETURNS TEXT
                IMMUTABLE STRICT
                LANGUAGE PLRUST AS
            $$
                Ok(Some(String::from("swooper")))
            $$;
        "#;
        Spi::run(definition)?;

        let retval = Spi::get_one(
            r#"
            SELECT update_me();
        "#,
        );
        assert_eq!(retval, Ok(Some("swooper")));
        Ok(())
    }

    #[pg_test]
    #[search_path(@extschema@)]
    fn plrust_spi() -> spi::Result<()> {
        let random_definition = r#"
            CREATE FUNCTION random_contributor_pet() RETURNS TEXT
                STRICT
                LANGUAGE PLRUST AS
            $$
                Ok(Spi::get_one("SELECT name FROM contributors_pets ORDER BY random() LIMIT 1")?)
            $$;
        "#;
        Spi::run(random_definition)?;

        let retval = Spi::get_one::<String>(
            r#"
            SELECT random_contributor_pet();
        "#,
        );
        assert!(retval.is_ok());
        assert!(retval.unwrap().is_some());

        let specific_definition = r#"
            CREATE FUNCTION contributor_pet(name TEXT) RETURNS BIGINT
                STRICT
                LANGUAGE PLRUST AS
            $$
                use pgrx::IntoDatum;
                Ok(Spi::get_one_with_args(
                    "SELECT id FROM contributors_pets WHERE name = $1",
                    vec![(PgBuiltInOids::TEXTOID.oid(), name.into_datum())],
                )?)
            $$;
        "#;
        Spi::run(specific_definition)?;

        let retval = Spi::get_one::<i64>(
            r#"
            SELECT contributor_pet('Nami');
        "#,
        );
        assert_eq!(retval, Ok(Some(2)));
        Ok(())
    }
}
