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
    use pgrx::spi;

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

#[cfg(any(test, feature = "pg_test"))]
pub mod pg_test {
    use once_cell::sync::Lazy;
    use tempfile::{tempdir, TempDir};

    static WORK_DIR: Lazy<String> = Lazy::new(|| {
        let work_dir = tempdir().expect("Couldn't create tempdir");
        format!("plrust.work_dir='{}'", work_dir.path().display())
    });
    static LOG_LEVEL: &str = "plrust.tracing_level=trace";

    static PLRUST_ALLOWED_DEPENDENCIES_FILE_NAME: &str = "allowed_deps.toml";
    static PLRUST_ALLOWED_DEPENDENCIES_FILE_DIRECTORY: Lazy<TempDir> = Lazy::new(|| {
        use std::io::Write;
        let temp_allowed_deps_dir = tempdir().expect("Couldnt create tempdir");

        let file_path = temp_allowed_deps_dir
            .path()
            .join(PLRUST_ALLOWED_DEPENDENCIES_FILE_NAME);
        let mut allowed_deps = std::fs::File::create(&file_path).unwrap();
        allowed_deps
            .write_all(
                r#"
owo-colors = "=3.5.0"
tokio = { version = "=1.19.2", features = ["rt", "net"]}
plutonium = "*"
syn = { version = "=2.0.28", default-features = false }
rand = ["=0.8.3", { version = ">0.8.4, <0.8.6", features = ["getrandom"]}]
"#
                .as_bytes(),
            )
            .unwrap();

        temp_allowed_deps_dir
    });

    static PLRUST_ALLOWED_DEPENDENCIES: Lazy<String> = Lazy::new(|| {
        format!(
            "plrust.allowed_dependencies='{}'",
            PLRUST_ALLOWED_DEPENDENCIES_FILE_DIRECTORY
                .path()
                .join(PLRUST_ALLOWED_DEPENDENCIES_FILE_NAME)
                .to_str()
                .unwrap()
        )
    });

    pub fn setup(_options: Vec<&str>) {
        // perform one-off initialization when the pg_test framework starts
    }

    pub fn postgresql_conf_options() -> Vec<&'static str> {
        vec![
            &*WORK_DIR,
            &*LOG_LEVEL,
            &*PLRUST_ALLOWED_DEPENDENCIES,
            "shared_preload_libraries='plrust'",
        ]
    }
}
