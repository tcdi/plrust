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
    #[should_panic(expected = "issue78 works")]
    fn test_issue_78() -> spi::Result<()> {
        let sql = r#"CREATE OR REPLACE FUNCTION raise_error() RETURNS TEXT
                        IMMUTABLE STRICT
                        LANGUAGE PLRUST AS
                    $$
                        pgrx::error!("issue78 works");
                        Ok(Some("hi".to_string()))
                    $$;"#;
        Spi::run(sql)?;
        Spi::get_one::<String>("SELECT raise_error()")?;
        Ok(())
    }

    #[pg_test]
    fn test_issue_79() -> spi::Result<()> {
        let sql = r#"
            create or replace function fn1(i int) returns int strict language plrust as $$
                [code]
                notice!("{}", "fn1 started");
                let cmd = format!("select fn2({})", i);
                Spi::connect(|client|
                    {
                        client.select(&cmd, None, None);
                    });
                notice!("{}", "fn1 finished");
                Ok(Some(1))
            $$;

            create or replace function fn2(i int) returns int strict language plrust as $$
                [code]
                notice!("{}", "fn2 started");
                notice!("{}", "fn2 finished");
                Ok(Some(2))
            $$;
        "#;
        Spi::run(sql)?;
        assert_eq!(Ok(Some(1)), Spi::get_one::<i32>("SELECT fn1(1)"));
        Ok(())
    }
}
