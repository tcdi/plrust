
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
    fn test_tid_roundtrip() -> spi::Result<()> {
        Spi::run(
            r#"CREATE FUNCTION tid_roundtrip(t tid) RETURNS tid LANGUAGE plrust AS $$ Ok(t) $$"#,
        )?;
        let tid = Spi::get_one::<pg_sys::ItemPointerData>("SELECT tid_roundtrip('(42, 99)'::tid)")?
            .expect("SPI result was null");
        let (blockno, offno) = pgrx::item_pointer_get_both(tid);
        assert_eq!(blockno, 42);
        assert_eq!(offno, 99);
        Ok(())
    }

    #[pg_test]
    fn test_return_bytea() -> spi::Result<()> {
        Spi::run(
            r#"CREATE FUNCTION return_bytea() RETURNS bytea LANGUAGE plrust AS $$ Ok(Some(vec![1,2,3])) $$"#,
        )?;
        let bytes = Spi::get_one::<Vec<u8>>("SELECT return_bytea()")?.expect("SPI result was null");
        assert_eq!(bytes, vec![1, 2, 3]);
        Ok(())
    }

    #[pg_test]
    fn test_cstring_roundtrip() -> Result<(), Box<dyn Error>> {
        use std::ffi::CStr;
        Spi::run(
            r#"CREATE FUNCTION cstring_roundtrip(s cstring) RETURNS cstring STRICT LANGUAGE plrust as $$ Ok(Some(s.into())) $$;"#,
        )?;
        let cstr = Spi::get_one::<&CStr>("SELECT cstring_roundtrip('hello')")?
            .expect("SPI result was null");
        let expected = CStr::from_bytes_with_nul(b"hello\0")?;
        assert_eq!(cstr, expected);
        Ok(())
    }

    #[pg_test]
    fn test_point() -> spi::Result<()> {
        Spi::run(
            r#"CREATE FUNCTION test_point(p point) RETURNS point LANGUAGE plrust AS $$ Ok(p) $$"#,
        )?;
        let p = Spi::get_one::<pg_sys::Point>("SELECT test_point('42, 99'::point);")?
            .expect("SPI result was null");
        assert_eq!(p.x, 42.0);
        assert_eq!(p.y, 99.0);
        Ok(())
    }

    #[pg_test]
    fn test_box() -> spi::Result<()> {
        Spi::run(r#"CREATE FUNCTION test_box(b box) RETURNS box LANGUAGE plrust AS $$ Ok(b) $$"#)?;
        let b = Spi::get_one::<pg_sys::BOX>("SELECT test_box('1,2,3,4'::box);")?
            .expect("SPI result was null");
        assert_eq!(b.high.x, 3.0);
        assert_eq!(b.high.y, 4.0);
        assert_eq!(b.low.x, 1.0);
        assert_eq!(b.low.y, 2.0);
        Ok(())
    }

    #[pg_test]
    fn test_uuid() -> spi::Result<()> {
        Spi::run(
            r#"CREATE FUNCTION test_uuid(u uuid) RETURNS uuid LANGUAGE plrust AS $$ Ok(u) $$"#,
        )?;
        let u = Spi::get_one::<pgrx::Uuid>(
            "SELECT test_uuid('e4176a4d-790c-4750-85b7-665d72471173'::uuid);",
        )?
            .expect("SPI result was null");
        assert_eq!(
            u,
            pgrx::Uuid::from_bytes([
                0xe4, 0x17, 0x6a, 0x4d, 0x79, 0x0c, 0x47, 0x50, 0x85, 0xb7, 0x66, 0x5d, 0x72, 0x47,
                0x11, 0x73
            ])
        );

        Ok(())
    }

}