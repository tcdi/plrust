mod basic;
mod versioning;
mod dependencies;
mod return_values;
mod ddl;
mod blocked_code;
mod recursion;
mod matches;
mod argument;
mod range;
mod user_defined_types;
mod time_and_dates;
mod borrow_mut_error;
mod panics;
mod alter;
mod round_trip;
mod trusted;

use pgrx::prelude::*;

pgrx::pg_module_magic!();

/*
Portions Copyright 2020-2021 ZomboDB, LLC.
Portions Copyright 2021-2023 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use pgrx::{prelude::*};

    // Bootstrap a testing table for non-immutable functions
    extension_sql!(
        r#"
        CREATE TABLE contributors_pets (
            id serial8 not null primary key,
            name text
        );
        INSERT INTO contributors_pets (name) VALUES ('Brandy');
        INSERT INTO contributors_pets (name) VALUES ('Nami');
        INSERT INTO contributors_pets (name) VALUES ('Sally');
        INSERT INTO contributors_pets (name) VALUES ('Anchovy');
    "#,
        name = "create_contributors_pets",
    );
}

#[cfg(any(test, feature = "pg_test"))]
pub mod pg_test {
    use once_cell::sync::Lazy;
    use tempdir::TempDir;

    static WORK_DIR: Lazy<String> = Lazy::new(|| {
        let work_dir = TempDir::new("plrust-tests").expect("Couldn't create tempdir");
        format!("plrust.work_dir='{}'", work_dir.path().display())
    });
    static LOG_LEVEL: &str = "plrust.tracing_level=trace";

    static PLRUST_ALLOWED_DEPENDENCIES_FILE_NAME: &str = "allowed_deps.toml";
    static PLRUST_ALLOWED_DEPENDENCIES_FILE_DIRECTORY: Lazy<TempDir> = Lazy::new(|| {
        use std::io::Write;
        let temp_allowed_deps_dir =
            TempDir::new("plrust-allowed-deps").expect("Couldnt create tempdir");

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

