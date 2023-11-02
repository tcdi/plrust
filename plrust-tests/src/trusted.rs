/*
Portions Copyright 2020-2021 ZomboDB, LLC.
Portions Copyright 2021-2023 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    #[allow(unused)]
    use pgrx::prelude::*;

    #[cfg(feature = "trusted")]
    #[pg_test]
    #[search_path(@extschema@)]
    fn postgrestd_dont_make_files() -> spi::Result<()> {
        let definition = r#"
                CREATE FUNCTION make_file(filename TEXT) RETURNS TEXT
                LANGUAGE PLRUST AS
                $$
                    Ok(std::fs::File::create(filename.unwrap_or("/somewhere/files/dont/belong.txt"))
                        .err()
                        .map(|e| e.to_string()))
                $$;
            "#;
        Spi::run(definition)?;

        let retval = Spi::get_one_with_args::<String>(
            r#"
                SELECT make_file($1);
            "#,
            vec![(
                PgBuiltInOids::TEXTOID.oid(),
                "/an/evil/place/to/put/a/file.txt".into_datum(),
            )],
        );
        assert_eq!(
            retval,
            Ok(Some("operation not supported on this platform".to_string()))
        );
        Ok(())
    }

    #[cfg(feature = "trusted")]
    #[pg_test]
    #[search_path(@extschema@)]
    #[should_panic = "Failed to execute command"]
    fn postgrestd_subprocesses_panic() -> spi::Result<()> {
        let definition = r#"
            CREATE FUNCTION say_hello()
            RETURNS text AS
            $$
                let out = std::process::Command::new("echo")
                    .arg("Hello world")
                    .stdout(std::process::Stdio::piped())
                    .output()
                    .expect("Failed to execute command");
                Ok(Some(String::from_utf8_lossy(&out.stdout).to_string()))
            $$ LANGUAGE plrust;
        "#;
        Spi::run(definition)?;

        let retval = Spi::get_one::<String>("SELECT say_hello();\n");
        assert_eq!(retval, Ok(Some("Hello world\n".into())));
        Ok(())
    }

    #[cfg(feature = "trusted")]
    #[pg_test]
    #[search_path(@extschema@)]
    #[should_panic = "error: the `include_str`, `include_bytes`, and `include` macros are forbidden"]
    fn postgrestd_no_include_str() -> spi::Result<()> {
        let definition = r#"
            CREATE FUNCTION include_str()
            RETURNS text AS
            $$
                let s = include_str!("/etc/passwd");
                Ok(Some(s.into()))
            $$ LANGUAGE plrust;
        "#;
        Spi::run(definition)?;

        let retval = Spi::get_one::<String>("SELECT include_str();\n")?;
        assert_eq!(retval.unwrap(), "");
        Ok(())
    }

    #[pg_test]
    #[search_path(@extschema@)]
    #[cfg(feature = "trusted")]
    #[should_panic = "No such file or directory (os error 2)"]
    fn plrustc_include_exists_no_access() {
        // This file is created in CI and exists, but can only be accessed by
        // root. Check that the actual access is reported as file not found (we
        // should be ensuring that via
        // `PLRUSTC_USER_CRATE_ALLOWED_SOURCE_PATHS`). We don't need to gate
        // this test on CI, since the file is unlikely to exist outside of CI
        // (so the test will pass).
        let definition = r#"
            CREATE FUNCTION include_no_access()
            RETURNS text AS $$
                include!("/var/ci-stuff/secret_rust_files/const_foo.rs");
                Ok(Some(format!("{BAR}")))
            $$ LANGUAGE plrust;
        "#;
        Spi::run(definition).unwrap()
    }

    #[pg_test]
    #[search_path(@extschema@)]
    #[cfg(feature = "trusted")]
    #[should_panic = "No such file or directory (os error 2)"]
    fn plrustc_include_exists_external() {
        // This file is created in CI, exists, and can be accessed by anybody,
        // but the actual access is forbidden via
        // `PLRUSTC_USER_CRATE_ALLOWED_SOURCE_PATHS`. We don't need to gate this test on
        // CI, since the file is unlikely to exist outside of CI, so the test
        // will pass anyway.
        let definition = r#"
            CREATE FUNCTION include_exists_external()
            RETURNS text AS $$
                include!("/var/ci-stuff/const_bar.rs");
                Ok(Some(format!("{BAR}")))
            $$ LANGUAGE plrust;
        "#;
        Spi::run(definition).unwrap();
    }

    #[pg_test]
    #[search_path(@extschema@)]
    #[cfg(feature = "trusted")]
    #[should_panic = "No such file or directory (os error 2)"]
    fn plrustc_include_made_up() {
        // This file does not exist, and should be reported as such.
        let definition = r#"
            CREATE FUNCTION include_madeup()
            RETURNS int AS $$
                include!("/made/up/path/lol.rs");
                Ok(Some(1))
            $$ LANGUAGE plrust;
        "#;
        Spi::run(definition).unwrap();
    }

    #[pg_test]
    #[search_path(@extschema@)]
    #[cfg(feature = "trusted")]
    #[should_panic = "No such file or directory (os error 2)"]
    fn plrustc_include_path_traversal() {
        use std::path::PathBuf;
        let workdir = Spi::get_one::<String>("SHOW plrust.work_dir")
            .expect("Could not get plrust.work_dir")
            .unwrap();

        let wd: PathBuf = PathBuf::from(workdir)
            .canonicalize()
            .ok()
            .expect("Failed to canonicalize workdir");

        // Produce a path that looks like
        // `/allowed/path/here/../../../illegal/path/here` and check that it's
        // rejected, in order to ensure we are not succeptable to path traversal
        // attacks.
        let mut evil_path = wd.clone();
        for _ in wd.ancestors().skip(1) {
            evil_path.push("..");
        }
        debug_assert_eq!(
            evil_path
                .canonicalize()
                .ok()
                .expect("Failed to produce unpath")
                .to_str(),
            Some("/")
        );
        evil_path.push("var/ci-stuff/const_bar.rs");
        // This file does not exist, and should be reported as such.
        let definition = format!(
            r#"CREATE FUNCTION include_sneaky_traversal()
            RETURNS int AS $$
                include!({evil_path:?});
                Ok(Some(1))
            $$ LANGUAGE plrust;"#
        );
        Spi::run(&definition).unwrap();
    }

    #[pg_test]
    #[search_path(@extschema@)]
    #[should_panic] // = "error: the `env` and `option_env` macros are forbidden"]
    #[cfg(feature = "trusted")]
    fn plrust_block_env() -> spi::Result<()> {
        let definition = r#"
            CREATE FUNCTION get_path() RETURNS text AS $$
                let path = env!("PATH");
                Ok(Some(path.to_string()))
            $$ LANGUAGE plrust;
        "#;
        Spi::run(definition)
    }

    #[pg_test]
    #[search_path(@extschema@)]
    #[should_panic] // = "error: the `env` and `option_env` macros are forbidden"]
    #[cfg(feature = "trusted")]
    fn plrust_block_option_env() -> spi::Result<()> {
        let definition = r#"
            CREATE FUNCTION try_get_path() RETURNS text AS $$
                match option_env!("PATH") {
                    None => Ok(None),
                    Some(s) => Ok(Some(s.to_string()))
                }
            $$ LANGUAGE plrust;
        "#;
        Spi::run(definition)
    }

    #[cfg(feature = "trusted")]
    #[pg_test]
    #[search_path(@extschema@)]
    fn postgrestd_net_is_unsupported() -> spi::Result<()> {
        let sql = r#"
        create or replace function pt106() returns text
        IMMUTABLE STRICT
        LANGUAGE PLRUST AS
        $$
        [code]
        use std::net::TcpStream;

        Ok(TcpStream::connect("127.0.0.1:22").err().map(|e| e.to_string()))
        $$"#;
        Spi::run(sql)?;
        let string = Spi::get_one::<String>("SELECT pt106()")?.expect("Unconditional return");
        assert_eq!("operation not supported on this platform", &string);
        Ok(())
    }
}
