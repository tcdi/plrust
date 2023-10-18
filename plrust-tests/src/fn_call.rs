/*
Portions Copyright 2020-2021 ZomboDB, LLC.
Portions Copyright 2021-2023 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use pgrx::prelude::*;

    #[pg_test]
    #[search_path(@extschema@)]
    fn plrust_fn_call() -> spi::Result<()> {
        let sql = r#"
            CREATE FUNCTION dynamic_function(i int) RETURNS int LANGUAGE plrust AS $$ Ok(i) $$;
        
            CREATE FUNCTION test_plrust_fn_call() RETURNS int LANGUAGE plrust
            AS $$ 
                 let result = fn_call("dynamic_function", &[&Arg::Value(42i32)]);
                 
                 assert_eq!(result, Ok(Some(42i32)));
                 
                 Ok(None)
            $$;
            
            SELECT test_plrust_fn_call();
        "#;
        Spi::run(sql)
    }
}
