/*
Portions Copyright 2020-2021 ZomboDB, LLC.
Portions Copyright 2021-2025 Technology Concepts & Design, Inc.

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE file.
*/

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use pgrx::prelude::*;

    #[pg_test]
    #[search_path(@extschema@)]
    fn plrust_aggregate() -> spi::Result<()> {
        let definition = r#"
            CREATE FUNCTION plrust_sum_state(state INT, next INT) RETURNS INT
                IMMUTABLE STRICT
                LANGUAGE PLRUST AS
            $$
                Ok(Some(state + next))
            $$;
            CREATE AGGREGATE plrust_sum(INT)
            (
                SFUNC    = plrust_sum_state,
                STYPE    = INT,
                INITCOND = '0'
            );
        "#;
        Spi::run(definition)?;

        let retval = Spi::get_one::<i32>(
            r#"
            SELECT plrust_sum(value) FROM UNNEST(ARRAY [1, 2, 3]) as value;
        "#,
        );
        assert_eq!(retval, Ok(Some(6)));
        Ok(())
    }

    #[pg_test]
    #[search_path(@extschema@)]
    fn plrust_trigger() -> spi::Result<()> {
        let definition = r#"
            CREATE TABLE dogs (
                name TEXT,
                scritches INT NOT NULL DEFAULT 0
            );

            CREATE FUNCTION pet_trigger() RETURNS trigger AS $$
                let mut new = trigger.new().unwrap().into_owned();

                let field = "scritches";

                match new.get_by_name::<i32>(field)? {
                    Some(val) => new.set_by_name(field, val + 1)?,
                    None => (),
                }

                Ok(Some(new))
            $$ LANGUAGE plrust;

            CREATE TRIGGER pet_trigger BEFORE INSERT OR UPDATE ON dogs
                FOR EACH ROW EXECUTE FUNCTION pet_trigger();

            INSERT INTO dogs (name) VALUES ('Nami');
        "#;
        Spi::run(definition)?;

        let retval = Spi::get_one::<i32>(
            r#"
            SELECT scritches FROM dogs;
        "#,
        );
        assert_eq!(retval, Ok(Some(1)));
        Ok(())
    }

    #[pg_test]
    fn replace_function() -> spi::Result<()> {
        Spi::run("CREATE FUNCTION replace_me() RETURNS int LANGUAGE plrust AS $$ Ok(Some(1)) $$")?;
        assert_eq!(Ok(Some(1)), Spi::get_one("SELECT replace_me()"));

        Spi::run(
            "CREATE OR REPLACE FUNCTION replace_me() RETURNS int LANGUAGE plrust AS $$ Ok(Some(2)) $$",
        )?;
        assert_eq!(Ok(Some(2)), Spi::get_one("SELECT replace_me()"));
        Ok(())
    }
}
