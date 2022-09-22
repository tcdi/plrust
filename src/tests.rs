#[cfg(any(test, feature = "pg_test"))]
#[pgx::pg_schema]
mod tests {
    use pgx::*;

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

    #[pg_test]
    #[search_path(@extschema@)]
    fn plrust_basic() {
        let definition = r#"
            CREATE FUNCTION sum_array(a BIGINT[]) RETURNS BIGINT
                IMMUTABLE STRICT
                LANGUAGE PLRUST AS
            $$
                Some(a.into_iter().map(|v| v.unwrap_or_default()).sum())
            $$;
        "#;
        Spi::run(definition);

        let retval = Spi::get_one_with_args(
            r#"
            SELECT sum_array($1);
        "#,
            vec![(
                PgBuiltInOids::INT4ARRAYOID.oid(),
                vec![1, 2, 3].into_datum(),
            )],
        );
        assert_eq!(retval, Some(6));
    }

    #[pg_test]
    #[search_path(@extschema@)]
    fn plrust_update() {
        let definition = r#"
            CREATE FUNCTION update_me() RETURNS TEXT
                IMMUTABLE STRICT
                LANGUAGE PLRUST AS
            $$
                String::from("booper").into()
            $$;
        "#;
        Spi::run(definition);

        let retval = Spi::get_one(
            r#"
            SELECT update_me();
        "#,
        );
        assert_eq!(retval, Some("booper"));

        let definition = r#"
            CREATE OR REPLACE FUNCTION update_me() RETURNS TEXT
                IMMUTABLE STRICT
                LANGUAGE PLRUST AS
            $$
                String::from("swooper").into()
            $$;
        "#;
        Spi::run(definition);

        let retval = Spi::get_one(
            r#"
            SELECT update_me();
        "#,
        );
        assert_eq!(retval, Some("swooper"));
    }

    #[pg_test]
    #[search_path(@extschema@)]
    fn plrust_spi() {
        let random_definition = r#"
            CREATE FUNCTION random_contributor_pet() RETURNS TEXT
                STRICT
                LANGUAGE PLRUST AS
            $$
                let name = Spi::get_one("SELECT name FROM contributors_pets ORDER BY random() LIMIT 1");
                name
            $$;
        "#;
        Spi::run(random_definition);

        let retval: Option<String> = Spi::get_one(
            r#"
            SELECT random_contributor_pet();
        "#,
        );
        assert!(retval.is_some());

        let specific_definition = r#"
            CREATE FUNCTION contributor_pet(name TEXT) RETURNS INT
                STRICT
                LANGUAGE PLRUST AS
            $$
                use pgx::IntoDatum;
                let id = Spi::get_one_with_args(
                    "SELECT id FROM contributors_pets WHERE name = $1",
                    vec![(PgBuiltInOids::TEXTOID.oid(), name.into_datum())],
                );
                id
            $$;
        "#;
        Spi::run(specific_definition);

        let retval: Option<i32> = Spi::get_one(
            r#"
            SELECT contributor_pet('Nami');
        "#,
        );
        assert_eq!(retval, Some(2));
    }

    #[pg_test]
    #[cfg(not(feature = "sandboxed"))]
    #[search_path(@extschema@)]
    fn plrust_deps() {
        let definition = r#"
                CREATE FUNCTION colorize(input TEXT) RETURNS TEXT
                IMMUTABLE STRICT
                LANGUAGE PLRUST AS
            $$
            [dependencies]
                owo-colors = "3"
            [code]
                use owo_colors::OwoColorize;
                Some(input.purple().to_string())
            $$;
        "#;
        Spi::run(definition);

        let retval: Option<String> = Spi::get_one_with_args(
            r#"
            SELECT colorize($1);
        "#,
            vec![(PgBuiltInOids::TEXTOID.oid(), "Nami".into_datum())],
        );
        assert!(retval.is_some());

        // Regression test: A previous version of PL/Rust would abort if this was called twice, so call it twice:
        let retval: Option<String> = Spi::get_one_with_args(
            r#"
            SELECT colorize($1);
        "#,
            vec![(PgBuiltInOids::TEXTOID.oid(), "Nami".into_datum())],
        );
        assert!(retval.is_some());
    }

    #[pg_test]
    #[search_path(@extschema@)]
    fn plrust_returns_setof() {
        let definition = r#"
            CREATE OR REPLACE FUNCTION boop_srf(names TEXT[]) RETURNS SETOF TEXT
                IMMUTABLE STRICT
                LANGUAGE PLRUST AS
            $$
                Some(::pgx::iter::SetOfIterator::new(names.into_iter().map(|maybe| maybe.map(|name| name.to_string() + " was booped!"))))
            $$;
        "#;
        Spi::run(definition);

        let retval = Spi::connect(|client| {
            let mut table = client.select(
                "SELECT * FROM boop_srf(ARRAY['Nami', 'Brandy'])",
                None,
                None,
            );

            let mut found = vec![];
            while table.next().is_some() {
                let value = table.get_one::<String>();
                found.push(value)
            }

            Ok(Some(found))
        });

        assert_eq!(
            retval,
            Some(vec![
                Some("Nami was booped!".into()),
                Some("Brandy was booped!".into()),
            ])
        );
    }

    #[pg_test]
    #[search_path(@extschema@)]
    fn plrust_aggregate() {
        let definition = r#"
            CREATE FUNCTION plrust_sum_state(state INT, next INT) RETURNS INT
                IMMUTABLE STRICT
                LANGUAGE PLRUST AS
            $$
                Some(state + next)
            $$;
            CREATE AGGREGATE plrust_sum(INT)
            (
                SFUNC    = plrust_sum_state,
                STYPE    = INT,
                INITCOND = '0'
            );            
        "#;
        Spi::run(definition);

        let retval: Option<i32> = Spi::get_one(
            r#"
            SELECT plrust_sum(value) FROM UNNEST(ARRAY [1, 2, 3]) as value;
        "#,
        );
        assert_eq!(retval, Some(6));
    }

    #[pg_test]
    #[search_path(@extschema@)]
    fn plrust_trigger() {
        let definition = r#"
            CREATE TABLE dogs (
                name TEXT,
                scritches INT NOT NULL DEFAULT 0
            );
            
            CREATE FUNCTION pet_trigger() RETURNS trigger AS $$
                let current = trigger.current().unwrap();
                let mut current = current.into_owned();
            
                let field = "scritches";
            
                match current.get_by_name::<i32>(field).unwrap() {
                    Some(val) => current.set_by_name(field, val + 1).unwrap(),
                    None => (),
                }
            
                Ok(current)
            $$ LANGUAGE plrust;
            
            CREATE TRIGGER pet_trigger BEFORE INSERT OR UPDATE ON dogs
                FOR EACH ROW EXECUTE FUNCTION pet_trigger();
            
            INSERT INTO dogs (name) VALUES ('Nami');     
        "#;
        Spi::run(definition);

        let retval: Option<i32> = Spi::get_one(
            r#"
            SELECT scritches FROM dogs;
        "#,
        );
        assert_eq!(retval, Some(1));
    }

    #[pg_test]
    #[search_path(@extschema@)]
    fn postgrestd_dont_make_files() {
        let definition = r#"
                CREATE FUNCTION make_file(filename TEXT) RETURNS TEXT
                LANGUAGE PLRUST AS
                $$
                    std::fs::File::create(filename.unwrap_or("/somewhere/files/dont/belong.txt"))
                        .err()
                        .map(|e| e.to_string())
                $$;
            "#;
        Spi::run(definition);

        let retval: Option<String> = Spi::get_one_with_args(
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
            Some("operation not supported on this platform".to_string())
        );
    }

    #[pg_test]
    #[search_path(@extschema@)]
    #[should_panic]
    fn pgx_can_panic() {
        panic!()
    }

    #[pg_test]
    #[search_path(@extschema@)]
    #[should_panic]
    fn plrust_can_panic() {
        let definition = r#"
            CREATE FUNCTION shut_up_and_explode()
            RETURNS text AS
            $$
                panic!();
                None
            $$ LANGUAGE plrust;
        "#;

        Spi::run(definition);
        let retval = Spi::get_one::<String>("SELECT shut_up_and_explode();\n");
        assert_eq!(retval, None);
    }

    #[pg_test]
    #[search_path(@extschema@)]
    #[should_panic]
    fn postgrestd_subprocesses_panic() {
        let definition = r#"
            CREATE FUNCTION say_hello()
            RETURNS text AS
            $$
                let out = std::process::Command::new("echo")
                    .arg("Hello world")
                    .stdout(std::process::Stdio::piped())
                    .output()
                    .expect("Failed to execute command");
                Some(String::from_utf8_lossy(&out.stdout).to_string())
            $$ LANGUAGE plrust;
        "#;
        Spi::run(definition);

        let retval = Spi::get_one::<String>("SELECT say_hello();\n");
        assert_eq!(retval, Some("Hello world\n".into()));
    }

    /// This test is... meta. It's intended to sleep enough to usually go last.
    /// If a previous test panics in a way so severe it aborts Postgres?
    /// This test will fail as a result.
    /// Ideally this should be slow but still sometimes finish second-to-last.
    #[pg_test]
    #[search_path(@extschema@)]
    fn plrust_one_sleepy_boi() {
        use std::{thread::sleep, time::Duration};
        let moment = Duration::from_secs(2);
        sleep(moment);

        let definition = r#"
            CREATE FUNCTION snooze()
            RETURNS text AS
            $$
                use std::{thread::sleep, time::{Duration, Instant}};

                let moment = Duration::from_secs(2);
                let now = Instant::now();

                sleep(moment);

                assert!(now.elapsed() >= moment);
                Some("zzz".into())
            $$ LANGUAGE plrust;
        "#;
        Spi::run(definition);
        sleep(moment);

        let retval = Spi::get_one::<String>("SELECT snooze();\n");
        sleep(moment);
        assert_eq!(retval, Some("zzz".into()));
    }
}

#[cfg(any(test, feature = "pg_test"))]
pub mod pg_test {
    use once_cell::sync::Lazy;
    use pgx_pg_config::Pgx;
    use tempdir::TempDir;

    static WORK_DIR: Lazy<String> = Lazy::new(|| {
        let work_dir = TempDir::new("plrust-tests").expect("Couldn't create tempdir");
        format!("plrust.work_dir='{}'", work_dir.path().display())
    });
    static PG_CONFIG: Lazy<String> = Lazy::new(|| {
        let pgx_config = Pgx::from_config().unwrap();
        let version = format!("pg{}", pgx::pg_sys::get_pg_major_version_num());
        let pg_config = pgx_config.get(&version).unwrap();
        let path = pg_config.path().unwrap();
        format!("plrust.pg_config='{}'", path.as_path().display())
    });
    static LOG_LEVEL: &str = "plrust.tracing_level=trace";

    pub fn setup(_options: Vec<&str>) {
        // perform one-off initialization when the pg_test framework starts
    }

    pub fn postgresql_conf_options() -> Vec<&'static str> {
        vec![&*WORK_DIR, &*PG_CONFIG, &*LOG_LEVEL]
    }
}
