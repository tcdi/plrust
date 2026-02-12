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
    fn plrust_call_1st() -> spi::Result<()> {
        let definition = r#"
            CREATE FUNCTION ret_1st(a int, b int)
            RETURNS int AS
            $$
                Ok(a)
            $$ LANGUAGE plrust;
        "#;
        Spi::run(definition)?;
        let result_1 = Spi::get_one::<i32>("SELECT ret_1st(1, 2);\n");
        assert_eq!(Ok(Some(1)), result_1); // may get: Some(1)
        Ok(())
    }

    #[pg_test]
    #[search_path(@extschema@)]
    fn plrust_call_2nd() -> spi::Result<()> {
        let definition = r#"
            CREATE FUNCTION ret_2nd(a int, b int)
            RETURNS int AS
            $$
                Ok(b)
            $$ LANGUAGE plrust;
        "#;
        Spi::run(definition)?;
        let result_2 = Spi::get_one::<i32>("SELECT ret_2nd(1, 2);\n");
        assert_eq!(Ok(Some(2)), result_2); // may get: Some(2)
        Ok(())
    }

    #[pg_test]
    #[search_path(@extschema@)]
    fn plrust_call_me() -> spi::Result<()> {
        let definition = r#"
            CREATE FUNCTION pick_ret(a int, b int, pick int)
            RETURNS int AS
            $$
                Ok(match pick {
                    Some(0) => a,
                    Some(1) => b,
                    _ => None,
                })
            $$ LANGUAGE plrust;
        "#;
        Spi::run(definition)?;
        let result_a = Spi::get_one::<i32>("SELECT pick_ret(3, 4, 0);");
        let result_b = Spi::get_one::<i32>("SELECT pick_ret(5, 6, 1);");
        let result_c = Spi::get_one::<i32>("SELECT pick_ret(7, 8, 2);");
        let result_z = Spi::get_one::<i32>("SELECT pick_ret(9, 99, -1);");
        assert_eq!(Ok(Some(3)), result_a); // may get: Some(4) or None
        assert_eq!(Ok(Some(6)), result_b); // may get: None
        assert_eq!(Ok(None), result_c);
        assert_eq!(Ok(None), result_z);
        Ok(())
    }

    #[pg_test]
    #[search_path(@extschema@)]
    fn plrust_call_me_call_me() -> spi::Result<()> {
        let definition = r#"
            CREATE FUNCTION ret_1st(a int, b int)
            RETURNS int AS
            $$
                Ok(a)
            $$ LANGUAGE plrust;

            CREATE FUNCTION ret_2nd(a int, b int)
            RETURNS int AS
            $$
                Ok(b)
            $$ LANGUAGE plrust;

            CREATE FUNCTION pick_ret(a int, b int, pick int)
            RETURNS int AS
            $$
                Ok(match pick {
                    Some(0) => a,
                    Some(1) => b,
                    _ => None,
                })
            $$ LANGUAGE plrust;
        "#;
        Spi::run(definition)?;
        let result_1 = Spi::get_one::<i32>("SELECT ret_1st(1, 2);\n");
        let result_2 = Spi::get_one::<i32>("SELECT ret_2nd(1, 2);\n");
        let result_a = Spi::get_one::<i32>("SELECT pick_ret(3, 4, 0);");
        let result_b = Spi::get_one::<i32>("SELECT pick_ret(5, 6, 1);");
        let result_c = Spi::get_one::<i32>("SELECT pick_ret(7, 8, 2);");
        let result_z = Spi::get_one::<i32>("SELECT pick_ret(9, 99, -1);");
        assert_eq!(Ok(None), result_z);
        assert_eq!(Ok(None), result_c);
        assert_eq!(Ok(Some(6)), result_b); // may get: None
        assert_eq!(Ok(Some(3)), result_a); // may get: Some(4) or None
        assert_eq!(Ok(Some(2)), result_2); // may get: Some(1)
        assert_eq!(Ok(Some(1)), result_1); // may get: Some(2)
        Ok(())
    }
}
