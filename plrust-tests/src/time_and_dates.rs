
/*
Portions Copyright 2020-2021 ZomboDB, LLC.
Portions Copyright 2021-2023 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use pgrx:: prelude::*;
    use std::error::Error;

    #[pg_test]
    fn test_daterange() -> Result<(), Box<dyn Error>> {
        Spi::run(
            r#"CREATE FUNCTION test_daterange(r daterange) RETURNS daterange LANGUAGE plrust AS $$ Ok(r) $$"#,
        )?;
        let r = Spi::get_one::<Range<Date>>(
            "SELECT test_daterange('[1977-03-20, 1980-01-01)'::daterange);",
        )?
            .expect("SPI result was null");
        assert_eq!(
            r,
            Range::new(
                Date::new(1977, 3, 20)?,
                RangeBound::Exclusive(Date::new(1980, 01, 01)?)
            )
        );
        Ok(())
    }

    #[pg_test]
    fn test_tsrange() -> Result<(), Box<dyn Error>> {
        Spi::run(
            r#"CREATE FUNCTION test_tsrange(p tsrange) RETURNS tsrange LANGUAGE plrust AS $$ Ok(p) $$"#,
        )?;
        let r = Spi::get_one::<Range<Timestamp>>(
            "SELECT test_tsrange('[1977-03-20, 1980-01-01)'::tsrange);",
        )?
            .expect("SPI result was null");
        assert_eq!(
            r,
            Range::new(
                Timestamp::new(1977, 3, 20, 0, 0, 0.0)?,
                RangeBound::Exclusive(Timestamp::new(1980, 01, 01, 0, 0, 0.0)?)
            )
        );
        Ok(())
    }

    #[pg_test]
    fn test_tstzrange() -> Result<(), Box<dyn Error>> {
        Spi::run(
            r#"CREATE FUNCTION test_tstzrange(p tstzrange) RETURNS tstzrange LANGUAGE plrust AS $$ Ok(p) $$"#,
        )?;
        let r = Spi::get_one::<Range<TimestampWithTimeZone>>(
            "SELECT test_tstzrange('[1977-03-20, 1980-01-01)'::tstzrange);",
        )?
            .expect("SPI result was null");
        assert_eq!(
            r,
            Range::new(
                TimestampWithTimeZone::new(1977, 3, 20, 0, 0, 0.0)?,
                RangeBound::Exclusive(TimestampWithTimeZone::new(1980, 01, 01, 0, 0, 0.0)?)
            )
        );
        Ok(())
    }

    #[pg_test]
    fn test_interval() -> Result<(), Box<dyn Error>> {
        Spi::run(
            r#"CREATE FUNCTION get_interval_hours(i interval) RETURNS numeric STRICT LANGUAGE plrust AS $$ Ok(i.extract_part(DateTimeParts::Hour)) $$"#,
        )?;
        let hours =
            Spi::get_one::<AnyNumeric>("SELECT get_interval_hours('3 days 9 hours 12 seconds')")?
                .expect("SPI result was null");
        assert_eq!(hours, AnyNumeric::from(9));
        Ok(())
    }

}