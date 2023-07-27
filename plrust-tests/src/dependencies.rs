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
    #[cfg(not(feature = "sandboxed"))]
    #[search_path(@extschema@)]
    fn plrust_deps_supported() -> spi::Result<()> {
        let definition = r#"
                CREATE FUNCTION colorize(input TEXT) RETURNS TEXT
                IMMUTABLE STRICT
                LANGUAGE PLRUST AS
            $$
            [dependencies]
                owo-colors = "3"
            [code]
                use owo_colors::OwoColorize;
                Ok(Some(input.purple().to_string()))
            $$;
        "#;
        Spi::run(definition)?;

        let retval = Spi::get_one_with_args::<String>(
            r#"
            SELECT colorize($1);
        "#,
            vec![(PgBuiltInOids::TEXTOID.oid(), "Nami".into_datum())],
        );
        assert!(retval.is_ok());
        assert!(retval.unwrap().is_some());

        // Regression test: A previous version of PL/Rust would abort if this was called twice, so call it twice:
        let retval = Spi::get_one_with_args::<String>(
            r#"
            SELECT colorize($1);
        "#,
            vec![(PgBuiltInOids::TEXTOID.oid(), "Nami".into_datum())],
        );
        assert!(retval.is_ok());
        assert!(retval.unwrap().is_some());
        Ok(())
    }

    #[pg_test]
    #[cfg(not(feature = "sandboxed"))]
    #[search_path(@extschema@)]
    fn plrust_deps_supported_deps_in_toml_table() -> spi::Result<()> {
        let definition = r#"
                CREATE FUNCTION say_hello() RETURNS TEXT
                IMMUTABLE STRICT
                LANGUAGE PLRUST AS
            $$
            [dependencies]
                tokio = ">=1"
                owo-colors = "3"
            [code]
                Ok(Some("hello".to_string()))
            $$;
        "#;
        Spi::run(definition)?;

        let retval = Spi::get_one_with_args::<String>(
            r#"
            SELECT say_hello();
        "#,
            vec![(PgBuiltInOids::TEXTOID.oid(), "hello".into_datum())],
        );
        assert_eq!(retval, Ok(Some("hello".to_string())));
        Ok(())
    }

    #[pg_test]
    #[cfg(not(feature = "sandboxed"))]
    #[search_path(@extschema@)]
    fn plrust_deps_not_supported() {
        let definition = r#"
                CREATE FUNCTION colorize(input TEXT) RETURNS TEXT
                IMMUTABLE STRICT
                LANGUAGE PLRUST AS
            $$
            [dependencies]
                regex = "1.6.5"
            [code]
                Ok(Some("test"))
            $$;
        "#;
        let res = std::panic::catch_unwind(|| {
            Spi::run(definition).expect("SQL for plrust_deps_not_supported() failed")
        });
        assert!(res.is_err());
    }

    #[pg_test]
    fn test_allowed_dependencies() -> spi::Result<()> {
        // Given the allowed list looks like this:
        // owo-colors = "=3.5.0"
        // tokio = { version = "=1.19.2", features = ["rt", "net"] }
        // plutonium = "*"
        // syn = { version = "=2.0.28", default-features = false }
        // rand = ["=0.8.3", { version = ">0.8.4, <0.8.6", features = ["getrandom"] }]
        let query = "SELECT * FROM plrust.allowed_dependencies();";

        // The result will look like this:
        //     name    |    version     |  features   | default_features
        // ------------+----------------+-------------+------------------
        //  owo-colors | =3.5.0         | {}          | t
        //  plutonium  | *              | {}          | t
        //  rand       | =0.8.3         | {}          | t
        //  rand       | >0.8.4, <0.8.6 | {getrandom} | t
        //  syn        | =2.0.28        | {}          | f
        //  tokio      | =1.19.2        | {rt,net}    | t

        Spi::connect(|client| {
            let expected_names = vec!["owo-colors", "plutonium", "rand", "rand", "syn", "tokio"];
            let expected_versions = vec![
                "=3.5.0",
                "*",
                "=0.8.3",
                ">0.8.4, <0.8.6",
                "=2.0.28",
                "=1.19.2",
            ];
            let expected_features = vec![
                vec![],
                vec![],
                vec![],
                vec![String::from("getrandom")],
                vec![],
                vec![String::from("rt"), String::from("net")],
            ];
            let expected_default_features = vec![true, true, true, true, false, true];
            let expected_len = expected_names.len();

            let tup_table = client.select(query, None, None)?;

            assert_eq!(tup_table.len(), expected_len);

            for (i, row) in tup_table.into_iter().enumerate() {
                assert_eq!(
                    row["name"].value::<String>().unwrap(),
                    Some(expected_names[i].to_owned())
                );
                assert_eq!(
                    row["version"].value::<String>().unwrap(),
                    Some(expected_versions[i].to_owned())
                );
                assert_eq!(
                    row["features"].value::<Vec<String>>().unwrap(),
                    Some(expected_features[i].to_owned())
                );
                assert_eq!(
                    row["default_features"].value::<bool>().unwrap(),
                    Some(expected_default_features[i])
                );
            }

            Ok(())
        })
    }
}
