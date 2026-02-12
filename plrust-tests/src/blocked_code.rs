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
    #[search_path(@ extschema @)]
    #[should_panic = "error: usage of an `unsafe` block"]
    fn plrust_block_unsafe_annotated() -> spi::Result<()> {
        // PL/Rust should block creating obvious, correctly-annotated usage of unsafe code
        let definition = r#"
            CREATE FUNCTION naughty()
            RETURNS text AS
            $$
                use std::{os::raw as ffi, str, ffi::CStr};
                let int:u32 = 0xDEADBEEF;
                // Note that it is always safe to create a pointer.
                let ptr = int as *mut u64;
                // What is unsafe is dereferencing it
                let cstr = unsafe {
                    ptr.write(0x00_1BADC0DE_00);
                    CStr::from_ptr(ptr.cast::<ffi::c_char>())
                };
                Ok(str::from_utf8(cstr.to_bytes()).ok().map(|s| s.to_owned()))
            $$ LANGUAGE plrust;
        "#;
        Spi::run(definition)
    }

    #[pg_test]
    #[search_path(@ extschema @)]
    #[should_panic = "call to unsafe function is unsafe and requires unsafe block"]
    fn plrust_block_unsafe_hidden() -> spi::Result<()> {
        // PL/Rust should not allow hidden injection of unsafe code
        // that may rely on the way PGRX expands into `unsafe fn` to "sneak in"
        let definition = r#"
            CREATE FUNCTION naughty()
            RETURNS text AS
            $$
                use std::{os::raw as ffi, str, ffi::CStr};
                let int:u32 = 0xDEADBEEF;
                let ptr = int as *mut u64;
                ptr.write(0x00_1BADC0DE_00);
                let cstr = CStr::from_ptr(ptr.cast::<ffi::c_char>());
                Ok(str::from_utf8(cstr.to_bytes()).ok().map(|s| s.to_owned()))
            $$ LANGUAGE plrust;
        "#;
        Spi::run(definition)
    }

    #[pg_test]
    #[search_path(@extschema@)]
    #[should_panic = "error: usage of an `unsafe` block"]
    fn plrust_block_unsafe_plutonium() -> spi::Result<()> {
        let definition = r#"
            CREATE FUNCTION super_safe()
            RETURNS text AS
            $$
                [dependencies]
                plutonium = "*"

                [code]
                use std::{os::raw as ffi, str, ffi::CStr};
                use plutonium::safe;

                #[safe]
                fn super_safe() -> Option<String> {
                    let int: u32 = 0xDEADBEEF;
                    let ptr = int as *mut u64;
                    ptr.write(0x00_1BADC0DE_00);
                    let cstr = CStr::from_ptr(ptr.cast::<ffi::c_char>());
                    str::from_utf8(cstr.to_bytes()).ok().map(|s| s.to_owned())
                }

                Ok(super_safe())
            $$ LANGUAGE plrust;
        "#;
        Spi::run(definition)
    }

    #[pg_test]
    #[search_path(@extschema@)]
    #[should_panic(expected = "error: declaration of a function with `export_name`")]
    fn plrust_block_unsafe_export_name() -> spi::Result<()> {
        // A separate test covers #[no_mangle], but what about #[export_name]?
        // Same idea. This tries to collide with free, which may symbol clash,
        // or might override depending on how the OS and loader feel today.
        // Let's not leave it up to forces beyond our control.
        let definition = r#"
            CREATE OR REPLACE FUNCTION export_hacked_free() RETURNS BIGINT
            IMMUTABLE STRICT
            LANGUAGE PLRUST AS
            $$
                #[export_name = "free"]
                pub extern "C" fn own_free(ptr: *mut c_void) {
                    // the contents don't matter
                }

                Ok(Some(1))
            $$;
        "#;
        Spi::run(definition)?;
        let result = Spi::get_one::<i32>("SELECT export_hacked_free();\n");
        assert_eq!(Ok(Some(1)), result);
        Ok(())
    }

    #[pg_test]
    #[search_path(@extschema@)]
    #[should_panic(expected = "error: declaration of a static with `link_section`")]
    fn plrust_block_unsafe_link_section() -> spi::Result<()> {
        let definition = r#"
            CREATE OR REPLACE FUNCTION link_evil_section() RETURNS BIGINT
            IMMUTABLE STRICT
            LANGUAGE PLRUST AS
            $$
                #[link_section = ".init_array"]
                pub static INITIALIZE: &[u8; 136] = &GOGO;

                #[link_section = ".text"]
                pub static GOGO: [u8; 136] = [
                    72, 184, 1, 1, 1, 1, 1, 1, 1, 1, 80, 72, 184, 46, 99, 104, 111, 46, 114, 105, 1, 72, 49, 4,
                    36, 72, 137, 231, 106, 1, 254, 12, 36, 72, 184, 99, 102, 105, 108, 101, 49, 50, 51, 80, 72,
                    184, 114, 47, 116, 109, 112, 47, 112, 111, 80, 72, 184, 111, 117, 99, 104, 32, 47, 118, 97,
                    80, 72, 184, 115, 114, 47, 98, 105, 110, 47, 116, 80, 72, 184, 1, 1, 1, 1, 1, 1, 1, 1, 80,
                    72, 184, 114, 105, 1, 44, 98, 1, 46, 116, 72, 49, 4, 36, 49, 246, 86, 106, 14, 94, 72, 1,
                    230, 86, 106, 19, 94, 72, 1, 230, 86, 106, 24, 94, 72, 1, 230, 86, 72, 137, 230, 49, 210,
                    106, 59, 88, 15, 5,
                ];

                Ok(Some(1))
            $$;
        "#;
        Spi::run(definition)?;
        let result = Spi::get_one::<i32>("SELECT link_evil_section();\n");
        assert_eq!(Ok(Some(1)), result);
        Ok(())
    }

    #[pg_test]
    #[search_path(@extschema@)]
    #[should_panic(expected = "error: declaration of a `no_mangle` static")]
    fn plrust_block_unsafe_no_mangle() -> spi::Result<()> {
        let definition = r#"
            CREATE OR REPLACE FUNCTION not_mangled() RETURNS BIGINT
            IMMUTABLE STRICT
            LANGUAGE PLRUST AS
            $$
                #[no_mangle]
                #[link_section = ".init_array"]
                pub static INITIALIZE: &[u8; 136] = &GOGO;

                #[no_mangle]
                #[link_section = ".text"]
                pub static GOGO: [u8; 136] = [
                    72, 184, 1, 1, 1, 1, 1, 1, 1, 1, 80, 72, 184, 46, 99, 104, 111, 46, 114, 105, 1, 72, 49, 4,
                    36, 72, 137, 231, 106, 1, 254, 12, 36, 72, 184, 99, 102, 105, 108, 101, 49, 50, 51, 80, 72,
                    184, 114, 47, 116, 109, 112, 47, 112, 111, 80, 72, 184, 111, 117, 99, 104, 32, 47, 118, 97,
                    80, 72, 184, 115, 114, 47, 98, 105, 110, 47, 116, 80, 72, 184, 1, 1, 1, 1, 1, 1, 1, 1, 80,
                    72, 184, 114, 105, 1, 44, 98, 1, 46, 116, 72, 49, 4, 36, 49, 246, 86, 106, 14, 94, 72, 1,
                    230, 86, 106, 19, 94, 72, 1, 230, 86, 106, 24, 94, 72, 1, 230, 86, 72, 137, 230, 49, 210,
                    106, 59, 88, 15, 5,
                ];

                Ok(Some(1))
            $$;
        "#;
        Spi::run(definition)?;
        let result = Spi::get_one::<i32>("SELECT not_mangled();\n");
        assert_eq!(Ok(Some(1)), result);
        Ok(())
    }
}
