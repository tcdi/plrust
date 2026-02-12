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
    #[should_panic = "plrust functions cannot have their STRICT property altered"]
    fn plrust_cant_change_strict_off() -> spi::Result<()> {
        let definition = r#"
            CREATE FUNCTION cant_change_strict_off()
            RETURNS int
            LANGUAGE plrust
            AS $$ Ok(Some(1)) $$;
        "#;
        Spi::run(definition)?;
        Spi::run("ALTER FUNCTION cant_change_strict_off() CALLED ON NULL INPUT")
    }

    #[pg_test]
    #[search_path(@extschema@)]
    #[should_panic = "plrust functions cannot have their STRICT property altered"]
    fn plrust_cant_change_strict_on() -> spi::Result<()> {
        let definition = r#"
            CREATE FUNCTION cant_change_strict_on()
            RETURNS int
            LANGUAGE plrust
            AS $$ Ok(Some(1)) $$;
        "#;
        Spi::run(definition)?;
        Spi::run("ALTER FUNCTION cant_change_strict_on() RETURNS NULL ON NULL INPUT")
    }
}
