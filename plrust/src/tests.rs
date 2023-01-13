#[cfg(any(test, feature = "pg_test"))]
#[pgx::pg_schema]
mod tests {
    use pgx::{datum::IntoDatum, prelude::*};

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
    fn plrust_basic() -> spi::Result<()> {
        let definition = r#"
            CREATE FUNCTION sum_array(a BIGINT[]) RETURNS BIGINT
                IMMUTABLE STRICT
                LANGUAGE PLRUST AS
            $$
                Some(a.into_iter().map(|v| v.unwrap_or_default()).sum())
            $$;
        "#;
        Spi::run(definition)?;

        let retval = Spi::get_one_with_args::<i64>(
            r#"
            SELECT sum_array($1);
        "#,
            vec![(
                PgBuiltInOids::INT4ARRAYOID.oid(),
                vec![1, 2, 3].into_datum(),
            )],
        );
        assert_eq!(retval, Ok(Some(6)));
        Ok(())
    }

    #[pg_test]
    #[search_path(@extschema@)]
    fn plrust_update() -> spi::Result<()> {
        let definition = r#"
            CREATE FUNCTION update_me() RETURNS TEXT
                IMMUTABLE STRICT
                LANGUAGE PLRUST AS
            $$
                String::from("booper").into()
            $$;
        "#;
        Spi::run(definition)?;

        let retval = Spi::get_one(
            r#"
            SELECT update_me();
        "#,
        );
        assert_eq!(retval, Ok(Some("booper")));

        let definition = r#"
            CREATE OR REPLACE FUNCTION update_me() RETURNS TEXT
                IMMUTABLE STRICT
                LANGUAGE PLRUST AS
            $$
                String::from("swooper").into()
            $$;
        "#;
        Spi::run(definition)?;

        let retval = Spi::get_one(
            r#"
            SELECT update_me();
        "#,
        );
        assert_eq!(retval, Ok(Some("swooper")));
        Ok(())
    }

    #[pg_test]
    #[search_path(@extschema@)]
    fn plrust_spi() -> spi::Result<()> {
        let random_definition = r#"
            CREATE FUNCTION random_contributor_pet() RETURNS TEXT
                STRICT
                LANGUAGE PLRUST AS
            $$
                let name = Spi::get_one("SELECT name FROM contributors_pets ORDER BY random() LIMIT 1");
                name.expect("Spi statement failed")
            $$;
        "#;
        Spi::run(random_definition)?;

        let retval = Spi::get_one::<String>(
            r#"
            SELECT random_contributor_pet();
        "#,
        );
        assert!(retval.is_ok());
        assert!(retval.unwrap().is_some());

        let specific_definition = r#"
            CREATE FUNCTION contributor_pet(name TEXT) RETURNS BIGINT
                STRICT
                LANGUAGE PLRUST AS
            $$
                use pgx::IntoDatum;
                let id = Spi::get_one_with_args(
                    "SELECT id FROM contributors_pets WHERE name = $1",
                    vec![(PgBuiltInOids::TEXTOID.oid(), name.into_datum())],
                ).expect("Spi statement failed");
                id
            $$;
        "#;
        Spi::run(specific_definition)?;

        let retval = Spi::get_one::<i64>(
            r#"
            SELECT contributor_pet('Nami');
        "#,
        );
        assert_eq!(retval, Ok(Some(2)));
        Ok(())
    }

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
                Some(input.purple().to_string())
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
    fn plrust_deps_supported_semver_parse() -> spi::Result<()> {
        let definition = r#"
                CREATE FUNCTION colorize(input TEXT) RETURNS TEXT
                IMMUTABLE STRICT
                LANGUAGE PLRUST AS
            $$
            [dependencies]
                owo-colors = ">2"
            [code]
                use owo_colors::OwoColorize;
                Some(input.purple().to_string())
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
                Some("hello".to_string())
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
                Some("test")
            $$;
        "#;
        let res = std::panic::catch_unwind(|| {
            Spi::run(definition).expect("SQL for plrust_deps_not_supported() failed")
        });
        assert!(res.is_err());
    }

    #[pg_test]
    #[search_path(@extschema@)]
    fn plrust_returns_setof() -> spi::Result<()> {
        let definition = r#"
            CREATE OR REPLACE FUNCTION boop_srf(names TEXT[]) RETURNS SETOF TEXT
                IMMUTABLE STRICT
                LANGUAGE PLRUST AS
            $$
                Some(::pgx::iter::SetOfIterator::new(names.into_iter().map(|maybe| maybe.map(|name| name.to_string() + " was booped!"))))
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
    #[search_path(@extschema@)]
    fn plrust_aggregate() -> spi::Result<()> {
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
        Spi::run(definition)?;

        let retval = Spi::get_one::<i32>(
            r#"
            SELECT plrust_sum(value) FROM UNNEST(ARRAY [1, 2, 3]) as value;
        "#,
        );
        assert_eq!(retval, Ok(Some(6)));
        Ok(())
    }

    #[pg_test]
    #[search_path(@extschema@)]
    fn plrust_trigger() -> spi::Result<()> {
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
        Spi::run(definition)?;

        let retval = Spi::get_one::<i32>(
            r#"
            SELECT scritches FROM dogs;
        "#,
        );
        assert_eq!(retval, Ok(Some(1)));
        Ok(())
    }

    #[pg_test]
    #[search_path(@extschema@)]
    fn postgrestd_dont_make_files() -> spi::Result<()> {
        let definition = r#"
                CREATE FUNCTION make_file(filename TEXT) RETURNS TEXT
                LANGUAGE PLRUST AS
                $$
                    std::fs::File::create(filename.unwrap_or("/somewhere/files/dont/belong.txt"))
                        .err()
                        .map(|e| e.to_string())
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

    #[pg_test]
    #[search_path(@extschema@)]
    #[should_panic]
    fn pgx_can_panic() {
        panic!()
    }

    #[pg_test]
    #[search_path(@extschema@)]
    #[should_panic]
    fn plrust_can_panic() -> spi::Result<()> {
        let definition = r#"
            CREATE FUNCTION shut_up_and_explode()
            RETURNS text AS
            $$
                panic!();
                None
            $$ LANGUAGE plrust;
        "#;

        Spi::run(definition)?;
        let retval = Spi::get_one::<String>("SELECT shut_up_and_explode();\n");
        assert_eq!(retval, Ok(None));
        Ok(())
    }

    #[pg_test]
    #[search_path(@extschema@)]
    #[should_panic]
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
                Some(String::from_utf8_lossy(&out.stdout).to_string())
            $$ LANGUAGE plrust;
        "#;
        Spi::run(definition)?;

        let retval = Spi::get_one::<String>("SELECT say_hello();\n");
        assert_eq!(retval, Ok(Some("Hello world\n".into())));
        Ok(())
    }

    /// This test is... meta. It's intended to sleep enough to usually go last.
    /// If a previous test panics in a way so severe it aborts Postgres?
    /// This test will fail as a result.
    /// Ideally this should be slow but still sometimes finish second-to-last.
    #[pg_test]
    #[search_path(@extschema@)]
    fn plrust_one_sleepy_boi() -> spi::Result<()> {
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

                // TODO: figure out why this isn't working
                // assert!(now.elapsed() >= moment);
                Some("zzz".into())
            $$ LANGUAGE plrust;
        "#;
        Spi::run(definition)?;
        sleep(moment);

        let retval = Spi::get_one::<String>("SELECT snooze();\n");
        sleep(moment);
        assert_eq!(retval, Ok(Some("zzz".into())));
        Ok(())
    }

    #[pg_test]
    #[search_path(@extschema@)]
    #[should_panic]
    fn plrust_block_unsafe_annotated() -> spi::Result<()> {
        // PL/Rust should block creating obvious, correctly-annotated usage of unsafe code
        let definition = r#"
            CREATE FUNCTION naughty()
            RETURNS text AS
            $$
                use std::{os::raw as ffi, str, ffi::CStr};
                let int = 0xDEADBEEF;
                // Note that it is always safe to create a pointer.
                let ptr = int as *mut u64;
                // What is unsafe is dereferencing it
                let cstr = unsafe {
                    ptr.write(0x00_1BADC0DE_00);
                    CStr::from_ptr(ptr.cast::<ffi::c_char>())
                };
                str::from_utf8(cstr.to_bytes()).ok().map(|s| s.to_owned())
            $$ LANGUAGE plrust;
        "#;
        Spi::run(definition)
    }

    #[pg_test]
    #[search_path(@extschema@)]
    #[should_panic]
    fn plrust_block_unsafe_hidden() -> spi::Result<()> {
        // PL/Rust should not allow hidden injection of unsafe code
        // that may rely on the way PGX expands into `unsafe fn` to "sneak in"
        let definition = r#"
            CREATE FUNCTION naughty()
            RETURNS text AS
            $$
                use std::{os::raw as ffi, str, ffi::CStr};
                let int = 0xDEADBEEF;
                let ptr = int as *mut u64;
                ptr.write(0x00_1BADC0DE_00);
                let cstr = CStr::from_ptr(ptr.cast::<ffi::c_char>());
                str::from_utf8(cstr.to_bytes()).ok().map(|s| s.to_owned())
            $$ LANGUAGE plrust;
        "#;
        Spi::run(definition)
    }

    #[pg_test]
    #[search_path(@extschema@)]
    #[should_panic]
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

                super_safe()
            $$ LANGUAGE plrust;
        "#;
        Spi::run(definition)
    }

    #[pg_test]
    #[search_path(@extschema@)]
    #[should_panic]
    fn plrust_pgloglevel_dont_allcaps_panic() -> spi::Result<()> {
        // This test attempts to annihilate the database.
        // It relies on the existing assumption that tests are run in the same Postgres instance,
        // so this test will make all tests "flaky" if Postgres suddenly goes down with it.
        let definition = r#"
            CREATE FUNCTION dont_allcaps_panic()
            RETURNS text AS
            $$
                use pgx::log::{PgLogLevel, elog};

                elog(PgLogLevel::PANIC, "If other tests completed, PL/Rust did not actually destroy the entire database, \
                                         But if you see this in the error output, something might be wrong.");
                Some("lol".into())
            $$ LANGUAGE plrust;
        "#;
        Spi::run(definition)?;
        let retval = Spi::get_one::<String>("SELECT dont_allcaps_panic();\n");
        assert_eq!(retval, Ok(Some("lol".into())));
        Ok(())
    }

    #[pg_test]
    #[search_path(@extschema@)]
    fn plrust_call_1st() -> spi::Result<()> {
        let definition = r#"
            CREATE FUNCTION ret_1st(a int, b int)
            RETURNS int AS
            $$
                a
            $$ LANGUAGE plrust;
        "#;
        Spi::run(definition)?;
        let result_1 = Spi::get_one::<i32>("SELECT ret_1st(1, 2);\n");
        assert_eq!(Ok(Some(1)), result_1); // may get: Some(1)
        Ok(())
    }

    #[pg_test]
    #[search_path(@extschema@)]
    fn plrust_call_2nd() -> spi::Result<()> {
        let definition = r#"
            CREATE FUNCTION ret_2nd(a int, b int)
            RETURNS int AS
            $$
                b
            $$ LANGUAGE plrust;
        "#;
        Spi::run(definition)?;
        let result_2 = Spi::get_one::<i32>("SELECT ret_2nd(1, 2);\n");
        assert_eq!(Ok(Some(2)), result_2); // may get: Some(2)
        Ok(())
    }

    #[pg_test]
    #[search_path(@extschema@)]
    fn plrust_call_me() -> spi::Result<()> {
        let definition = r#"
            CREATE FUNCTION pick_ret(a int, b int, pick int)
            RETURNS int AS
            $$
                match pick {
                    Some(0) => a,
                    Some(1) => b,
                    _ => None,
                }
            $$ LANGUAGE plrust;
        "#;
        Spi::run(definition)?;
        let result_a = Spi::get_one::<i32>("SELECT pick_ret(3, 4, 0);");
        let result_b = Spi::get_one::<i32>("SELECT pick_ret(5, 6, 1);");
        let result_c = Spi::get_one::<i32>("SELECT pick_ret(7, 8, 2);");
        let result_z = Spi::get_one::<i32>("SELECT pick_ret(9, 99, -1);");
        assert_eq!(Ok(Some(3)), result_a); // may get: Some(4) or None
        assert_eq!(Ok(Some(6)), result_b); // may get: None
        assert_eq!(Ok(None), result_c);
        assert_eq!(Ok(None), result_z);
        Ok(())
    }

    #[pg_test]
    #[search_path(@extschema@)]
    fn plrust_call_me_call_me() -> spi::Result<()> {
        let definition = r#"
            CREATE FUNCTION ret_1st(a int, b int)
            RETURNS int AS
            $$
                a
            $$ LANGUAGE plrust;

            CREATE FUNCTION ret_2nd(a int, b int)
            RETURNS int AS
            $$
                b
            $$ LANGUAGE plrust;

            CREATE FUNCTION pick_ret(a int, b int, pick int)
            RETURNS int AS
            $$
                match pick {
                    Some(0) => a,
                    Some(1) => b,
                    _ => None,
                }
            $$ LANGUAGE plrust;
        "#;
        Spi::run(definition)?;
        let result_1 = Spi::get_one::<i32>("SELECT ret_1st(1, 2);\n");
        let result_2 = Spi::get_one::<i32>("SELECT ret_2nd(1, 2);\n");
        let result_a = Spi::get_one::<i32>("SELECT pick_ret(3, 4, 0);");
        let result_b = Spi::get_one::<i32>("SELECT pick_ret(5, 6, 1);");
        let result_c = Spi::get_one::<i32>("SELECT pick_ret(7, 8, 2);");
        let result_z = Spi::get_one::<i32>("SELECT pick_ret(9, 99, -1);");
        assert_eq!(Ok(None), result_z);
        assert_eq!(Ok(None), result_c);
        assert_eq!(Ok(Some(6)), result_b); // may get: None
        assert_eq!(Ok(Some(3)), result_a); // may get: Some(4) or None
        assert_eq!(Ok(Some(2)), result_2); // may get: Some(1)
        assert_eq!(Ok(Some(1)), result_1); // may get: Some(2)
        Ok(())
    }

    #[pg_test]
    #[search_path(@extschema@)]
    #[should_panic]
    fn plrust_dup_args() -> spi::Result<()> {
        let definition = r#"
            CREATE FUNCTION not_unique(a int, a int)
            RETURNS int AS
            $$
                a
            $$ LANGUAGE plrust;
        "#;
        Spi::run(definition)?;
        let result = Spi::get_one::<i32>("SELECT not_unique(1, 2);\n");
        assert_eq!(Ok(Some(1)), result);
        Ok(())
    }

    #[pg_test]
    #[search_path(@extschema@)]
    #[should_panic]
    fn plrust_defaulting_dup_args() -> spi::Result<()> {
        let definition = r#"
            CREATE FUNCTION not_unique(int, arg0 int)
            RETURNS int AS
            $$
                arg0
            $$ LANGUAGE plrust;
        "#;
        Spi::run(definition)?;
        let result = Spi::get_one::<i32>("SELECT not_unique(1, 2);\n");
        assert_eq!(Ok(Some(1)), result);
        Ok(())
    }

    #[pg_test]
    #[search_path(@extschema@)]
    #[should_panic]
    fn plrust_cant_change_strict_off() -> spi::Result<()> {
        let definition = r#"
            CREATE FUNCTION cant_change_strict_off()
            RETURNS int
            LANGUAGE plrust
            AS $$ Some(1) $$;
        "#;
        Spi::run(definition)?;
        Spi::run("ALTER FUNCTION cant_change_strict() CALLED ON NULL INPUT")
    }

    #[pg_test]
    #[search_path(@extschema@)]
    #[should_panic]
    fn plrust_cant_change_strict_on() -> spi::Result<()> {
        let definition = r#"
            CREATE FUNCTION cant_change_strict_on()
            RETURNS int
            LANGUAGE plrust
            AS $$ Some(1) $$;
        "#;
        Spi::run(definition)?;
        Spi::run("ALTER FUNCTION cant_change_strict() RETURNS NULL ON NULL INPUT")
    }

    #[pg_test]
    #[search_path(@extschema@)]
    fn plrust_drop_function() -> spi::Result<()> {
        let definition = r#"
            CREATE FUNCTION drop_function()
            RETURNS int
            LANGUAGE plrust
            AS $$ Some(1) $$;
        "#;
        Spi::run(definition)?;
        let oid = Spi::get_one::<pg_sys::Oid>(
            "SELECT oid FROM pg_catalog.pg_proc WHERE proname = 'drop_function'",
        )?
        .expect("failed to lookup function oid")
        .as_u32();

        let procedure_id = pg_sys::ProcedureRelationId.as_u32();
        let identity = Spi::get_one::<String>(&format!(
            "SELECT identity from pg_identify_object({procedure_id}, {oid}, 0)",
        ))?
        .expect("call to pg_identify_object returned NULL");

        Spi::run("DROP FUNCTION drop_function")?;

        let our_id = Spi::get_one::<String>(&format!(
            "SELECT id FROM plrust.plrust_proc WHERE id = '{identity}'"
        ));
        assert_eq!(our_id, Err(spi::Error::InvalidPosition));
        Ok(())
    }

    #[pg_test]
    #[search_path(@extschema@)]
    fn plrust_drop_schema() -> spi::Result<()> {
        let definition = r#"
            CREATE SCHEMA to_drop;
            CREATE FUNCTION to_drop.drop_function()
            RETURNS int
            LANGUAGE plrust
            AS $$ Some(1) $$;
        "#;
        Spi::run(definition)?;
        let oid = Spi::get_one::<pg_sys::Oid>(
            "SELECT oid FROM pg_catalog.pg_proc WHERE oid = 'to_drop.drop_function'::regproc::oid",
        )?
        .expect("failed to lookup function oid")
        .as_u32();

        let procedure_id = pg_sys::ProcedureRelationId.as_u32();
        let identity = Spi::get_one::<String>(&format!(
            "SELECT identity from pg_identify_object({procedure_id}, {oid}, 0)"
        ))?
        .expect("call to pg_identify_object returned NULL");

        Spi::run("DROP SCHEMA to_drop CASCADE")?;
        let our_id = Spi::get_one::<String>(&format!(
            "SELECT id FROM plrust.plrust_proc WHERE id = '{identity}'"
        ));
        assert_eq!(our_id, Err(spi::Error::InvalidPosition));
        Ok(())
    }

    #[pg_test]
    #[search_path(@extschema@)]
    #[should_panic(expected = "error: declaration of a `no_mangle` static")]
    fn plrust_block_unsafe_no_mangle() -> spi::Result<()> {
        let definition = r#"
            CREATE OR REPLACE FUNCTION no_mangle() RETURNS BIGINT
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

                Some(1)
            $$;
        "#;
        Spi::run(definition)?;
        let result = Spi::get_one::<i32>("SELECT no_mangle();\n");
        assert_eq!(Ok(Some(1)), result);
        Ok(())
    }

    #[pg_test]
    #[search_path(@extschema@)]
    #[should_panic(expected = "error: declaration of a static with `link_section`")]
    fn plrust_block_unsafe_link_section() -> spi::Result<()> {
        let definition = r#"
            CREATE OR REPLACE FUNCTION link_section() RETURNS BIGINT
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

                Some(1)
            $$;
        "#;
        Spi::run(definition)?;
        let result = Spi::get_one::<i32>("SELECT link_section();\n");
        assert_eq!(Ok(Some(1)), result);
        Ok(())
    }

    #[pg_test]
    #[should_panic(expected = "issue78 works")]
    fn test_issue_78() -> spi::Result<()> {
        let sql = r#"CREATE OR REPLACE FUNCTION raise_error() RETURNS TEXT
                        IMMUTABLE STRICT
                        LANGUAGE PLRUST AS
                    $$
                        pgx::error!("issue78 works");
                        Some("hi".to_string())
                    $$;"#;
        Spi::run(sql)?;
        Spi::get_one::<String>("SELECT raise_error()")?;
        Ok(())
    }

    #[pg_test]
    fn test_issue_79() -> spi::Result<()> {
        let sql = r#"
            create or replace function fn1(i int) returns int strict language plrust as $$
                [code]
                notice!("{}", "fn1 started");
                let cmd = format!("select fn2({})", i);
                Spi::connect(|client|
                    {
                        client.select(&cmd, None, None);
                    });
                notice!("{}", "fn1 finished");
                Some(1)
            $$;

            create or replace function fn2(i int) returns int strict language plrust as $$
                [code]
                notice!("{}", "fn2 started");
                notice!("{}", "fn2 finished");
                Some(2)
            $$;
        "#;
        Spi::run(sql)?;
        assert_eq!(Ok(Some(1)), Spi::get_one::<i32>("SELECT fn1(1)"));
        Ok(())
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
                r#"owo-colors = "3.5.0"
tokio = { version = "1.19.2", features = ["rt", "net"]}"#
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
            &*PG_CONFIG,
            &*LOG_LEVEL,
            &*PLRUST_ALLOWED_DEPENDENCIES,
            "shared_preload_libraries='plrust'",
        ]
    }
}
