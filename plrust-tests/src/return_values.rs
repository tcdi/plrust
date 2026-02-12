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
    #[search_path(@ extschema @)]
    fn plrust_returns_setof() -> spi::Result<()> {
        let definition = r#"
            CREATE OR REPLACE FUNCTION boop_srf(names TEXT[]) RETURNS SETOF TEXT
                IMMUTABLE STRICT
                LANGUAGE PLRUST AS
            $$
                Ok(Some(::pgrx::iter::SetOfIterator::new(names.into_iter().map(|maybe| maybe.map(|name| name.to_string() + " was booped!")))))
            $$;
        "#;
        Spi::run(definition)?;

        let retval: spi::Result<_> = Spi::connect(|client| {
            let mut table = client.select(
                "SELECT * FROM boop_srf(ARRAY['Nami', 'Brandy'])",
                None,
                None,
            )?;

            let mut found = vec![];
            while table.next().is_some() {
                let value = table.get_one::<String>()?;
                found.push(value)
            }

            Ok(Some(found))
        });

        assert_eq!(
            retval,
            Ok(Some(vec![
                Some("Nami was booped!".into()),
                Some("Brandy was booped!".into()),
            ]))
        );
        Ok(())
    }

    #[pg_test]
    fn test_srf_one_col() -> spi::Result<()> {
        Spi::run(
            "CREATE FUNCTION srf_one_col() RETURNS TABLE (a int) LANGUAGE plrust AS $$
            Ok(Some(TableIterator::new(vec![( Some(1), )].into_iter())))
        $$;",
        )?;

        let a = Spi::get_one::<i32>("SELECT * FROM srf_one_col()")?;
        assert_eq!(a, Some(1));

        Ok(())
    }

    #[pg_test]
    fn test_srf_two_col() -> spi::Result<()> {
        Spi::run(
            "CREATE FUNCTION srf_two_col() RETURNS TABLE (a int, b int) LANGUAGE plrust AS $$
            Ok(Some(TableIterator::new(vec![(Some(1), Some(2))].into_iter())))
        $$;",
        )?;

        let (a, b) = Spi::get_two::<i32, i32>("SELECT * FROM srf_two_col()")?;
        assert_eq!(a, Some(1));
        assert_eq!(b, Some(2));

        Ok(())
    }
}
