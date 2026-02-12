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
    #[should_panic = "parameter name \"a\" used more than once"]
    fn plrust_dup_args() -> spi::Result<()> {
        let definition = r#"
            CREATE FUNCTION not_unique(a int, a int)
            RETURNS int AS
            $$
                Ok(a)
            $$ LANGUAGE plrust;
        "#;
        Spi::run(definition)?;
        let result = Spi::get_one::<i32>("SELECT not_unique(1, 2);\n");
        assert_eq!(Ok(Some(1)), result);
        Ok(())
    }

    #[pg_test]
    #[search_path(@extschema@)]
    #[should_panic = "PL/Rust does not support unnamed arguments"]
    fn plrust_defaulting_dup_args() -> spi::Result<()> {
        let definition = r#"
            CREATE FUNCTION not_unique(int, arg0 int)
            RETURNS int AS
            $$
                Ok(arg0)
            $$ LANGUAGE plrust;
        "#;
        Spi::run(definition)?;
        let result = Spi::get_one::<i32>("SELECT not_unique(1, 2);\n");
        assert_eq!(Ok(Some(1)), result);
        Ok(())
    }

    #[pg_test]
    #[search_path(@extschema@)]
    #[should_panic(expected = "PL/Rust does not support unnamed arguments")]
    fn unnamed_args() -> spi::Result<()> {
        Spi::run("CREATE FUNCTION unnamed_arg(int) RETURNS int LANGUAGE plrust as $$ Ok(None) $$;")
    }

    #[pg_test]
    #[search_path(@extschema@)]
    #[should_panic(expected = "PL/Rust does not support unnamed arguments")]
    fn named_unnamed_args() -> spi::Result<()> {
        Spi::run("CREATE FUNCTION named_unnamed_arg(bob text, int) RETURNS int LANGUAGE plrust as $$ Ok(None) $$;")
    }

    #[pg_test]
    #[search_path(@extschema@)]
    #[should_panic(
        expected = "is an invalid Rust identifier and cannot be used as an argument name"
    )]
    fn invalid_arg_identifier() -> spi::Result<()> {
        Spi::run("CREATE FUNCTION invalid_arg_identifier(\"this isn't a valid rust identifier\" int) RETURNS int LANGUAGE plrust as $$ Ok(None) $$;")
    }
}
