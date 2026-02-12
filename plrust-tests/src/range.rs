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
    fn test_int4range() -> spi::Result<()> {
        Spi::run(
            r#"CREATE FUNCTION test_int4range(r int4range) RETURNS int4range LANGUAGE plrust AS $$ Ok(r) $$"#,
        )?;
        let r = Spi::get_one::<Range<i32>>("SELECT test_int4range('[1, 10)'::int4range);")?
            .expect("SPI result was null");
        assert_eq!(r, (1..10).into());
        Ok(())
    }

    #[pg_test]
    fn test_int8range() -> spi::Result<()> {
        Spi::run(
            r#"CREATE FUNCTION test_int8range(r int8range) RETURNS int8range LANGUAGE plrust AS $$ Ok(r) $$"#,
        )?;
        let r = Spi::get_one::<Range<i64>>("SELECT test_int8range('[1, 10)'::int8range);")?
            .expect("SPI result was null");
        assert_eq!(r, (1..10).into());
        Ok(())
    }

    #[pg_test]
    fn test_numrange() -> spi::Result<()> {
        Spi::run(
            r#"CREATE FUNCTION test_numrange(r numrange) RETURNS numrange LANGUAGE plrust AS $$ Ok(r) $$"#,
        )?;
        let r = Spi::get_one::<Range<AnyNumeric>>("SELECT test_numrange('[1, 10]'::numrange);")?
            .expect("SPI result was null");
        assert_eq!(
            r,
            Range::new(
                AnyNumeric::try_from(1.0f32).unwrap(),
                AnyNumeric::try_from(10.0f32).unwrap()
            )
        );
        Ok(())
    }
}
