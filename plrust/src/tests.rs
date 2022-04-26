
#[cfg(any(test, feature = "pg_test"))]
#[pgx::pg_schema]
mod tests {
    use super::*;
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
    fn accepts_and_returns_text() {
        let definition = r#"
            CREATE OR REPLACE FUNCTION accepts_and_returns_text(input TEXT) RETURNS TEXT
                IMMUTABLE STRICT
                LANGUAGE PLRUST AS
            $$
                Ok(input)
            $$;
        "#;
        Spi::run(definition);

        let retval = Spi::get_one(
            r#"
            SELECT accepts_and_returns_text('booper');
        "#,
        );
        assert_eq!(retval, Some("booper"));
    }

    #[pg_test]
    #[search_path(@extschema@)]
    fn accepts_and_returns_text_list() {
        let definition = r#"
            CREATE OR REPLACE FUNCTION accepts_and_returns_text_list(input TEXT[]) RETURNS TEXT[]
                IMMUTABLE STRICT
                LANGUAGE PLRUST AS
            $$
                Ok(input)
            $$;
        "#;
        Spi::run(definition);

        let retval = Spi::get_one(
            r#"
            SELECT accepts_and_returns_text_list(ARRAY['Nami', 'Brandy']);
        "#,
        );
        pgx::warning!("retval from spi: {:?}", retval);

        assert_eq!(retval, Some(vec![Some("Nami"), Some("Brandy")]));
    }

    #[pg_test]
    #[search_path(@extschema@)]
    fn accepts_and_returns_int() {
        let definition = r#"
            CREATE OR REPLACE FUNCTION accepts_and_returns_int(input INT) RETURNS INT
                IMMUTABLE STRICT
                LANGUAGE PLRUST AS
            $$
                Ok(input)
            $$;
        "#;
        Spi::run(definition);

        let retval = Spi::get_one(
            r#"
            SELECT accepts_and_returns_int(1);
        "#,
        );
        assert_eq!(retval, Some(1));
    }

    #[pg_test]
    #[search_path(@extschema@)]
    fn accepts_and_returns_int_list() {
        let definition = r#"
            CREATE OR REPLACE FUNCTION accepts_and_returns_int_list(input INT[]) RETURNS INT[]
                IMMUTABLE STRICT
                LANGUAGE PLRUST AS
            $$
                Ok(input)
            $$;
        "#;
        Spi::run(definition);

        let retval = Spi::get_one(
            r#"
            SELECT accepts_and_returns_int_list(ARRAY[1, 2]);
        "#,
        );
        assert_eq!(retval, Some(vec![1, 2]));
    }


    #[pg_test]
    #[search_path(@extschema@)]
    fn accepts_and_returns_bigint() {
        let definition = r#"
            CREATE OR REPLACE FUNCTION accepts_and_returns_bigint(input BIGINT) RETURNS BIGINT
                IMMUTABLE STRICT
                LANGUAGE PLRUST AS
            $$
                Ok(input)
            $$;
        "#;
        Spi::run(definition);

        let retval = Spi::get_one(
            r#"
            SELECT accepts_and_returns_bigint(1);
        "#,
        );
        assert_eq!(retval, Some(1));
    }

    #[pg_test]
    #[search_path(@extschema@)]
    fn accepts_and_returns_bigint_list() {
        let definition = r#"
            CREATE OR REPLACE FUNCTION accepts_and_returns_bigint_list(input BIGINT[]) RETURNS BIGINT[]
                IMMUTABLE STRICT
                LANGUAGE PLRUST AS
            $$
                Ok(input)
            $$;
        "#;
        Spi::run(definition);

        let retval = Spi::get_one(
            r#"
            SELECT accepts_and_returns_bigint_list(ARRAY[1, 2]);
        "#,
        );
        assert_eq!(retval, Some(vec![1, 2]));
    }

    #[pg_test]
    #[search_path(@extschema@)]
    fn accepts_and_returns_bool() {
        let definition = r#"
            CREATE OR REPLACE FUNCTION accepts_and_returns_bool(input BOOL) RETURNS BOOL
                IMMUTABLE STRICT
                LANGUAGE PLRUST AS
            $$
                Ok(input)
            $$;
        "#;
        Spi::run(definition);

        let retval = Spi::get_one(
            r#"
            SELECT accepts_and_returns_bool(true);
        "#,
        );
        assert_eq!(retval, Some(true));
    }

    #[pg_test]
    #[search_path(@extschema@)]
    fn accepts_and_returns_bool_list() {
        let definition = r#"
            CREATE OR REPLACE FUNCTION accepts_and_returns_bool_list(input BOOL[]) RETURNS BOOL[]
                IMMUTABLE STRICT
                LANGUAGE PLRUST AS
            $$
                Ok(input)
            $$;
        "#;
        Spi::run(definition);

        let retval = Spi::get_one(
            r#"
            SELECT accepts_and_returns_bool_list(ARRAY[true, false]);
        "#,
        );
        assert_eq!(retval, Some(vec![true, false]));
    }

    #[pg_test]
    #[search_path(@extschema@)]
    fn accepts_multiple_args() {
        let definition = r#"
            CREATE OR REPLACE FUNCTION accepts_multiple_args(pet TEXT, food TEXT, times INT) RETURNS TEXT
                IMMUTABLE STRICT
                LANGUAGE PLRUST AS
            $$
                let pet = pet;
                let food = food;
                let times = times;
                Ok(format!("{} eats {} {} times.", pet, food, times))
            $$;
        "#;
        Spi::run(definition);

        let retval = Spi::get_one(
            r#"
            SELECT accepts_multiple_args('Nami', 'duck', '2');
        "#,
        );
        assert_eq!(retval, Some("Nami eats duck 2 times."));
    }

    // #[pg_test]
    // #[search_path(@extschema@)]
    // fn test_lists() {
    //     let definition = r#"
    //         CREATE OR REPLACE FUNCTION sum_array(a BIGINT[]) RETURNS BIGINT
    //             IMMUTABLE STRICT
    //             LANGUAGE PLRUST AS
    //         $$
    //             Ok(a.into_iter().map(|v| v.unwrap_or_default()).sum())
    //         $$;
    //     "#;
    //     Spi::run(definition);

    //     let retval = Spi::get_one_with_args(
    //         r#"
    //         SELECT sum_array($1);
    //     "#,
    //         vec![(
    //             PgBuiltInOids::INT4ARRAYOID.oid(),
    //             vec![1, 2, 3].into_datum(),
    //         )],
    //     );
    //     assert_eq!(retval, Some(6));
    // }

    #[pg_test]
    #[search_path(@extschema@)]
    fn update() {
        let definition = r#"
            CREATE OR REPLACE FUNCTION update_me() RETURNS TEXT
                IMMUTABLE STRICT
                LANGUAGE PLRUST AS
            $$
                Ok(String::from("booper"))
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

                Ok(String::from("swooper"))
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
    fn spi() {
        let random_definition = r#"
            CREATE OR REPLACE FUNCTION random_contributor_pet() RETURNS TEXT
                STRICT
                LANGUAGE PLRUST AS
            $$
                let name: String = interface::get_one(
                    "SELECT name FROM contributors_pets ORDER BY random() LIMIT 1",
                )?.unwrap();
                Ok(name)
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
            CREATE OR REPLACE FUNCTION contributor_pet(name TEXT) RETURNS INT
                STRICT
                LANGUAGE PLRUST AS
            $$
                let id: i32 = interface::get_one_with_args(
                    "SELECT id FROM contributors_pets WHERE name = $1",
                    &[name.as_str().into()],
                )?.unwrap();

                Ok(id)
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
    fn deps() {
        let definition = r#"
            CREATE OR REPLACE FUNCTION colorize(input TEXT) RETURNS TEXT
                IMMUTABLE STRICT
                LANGUAGE PLRUST AS
            $$
            [dependencies]
                owo-colors = "3"
            [code]
                use owo_colors::OwoColorize;

                Ok(input.purple().to_string())
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
    }
}

#[cfg(test)]
pub mod pg_test {
    use once_cell::sync::Lazy;
    use tempdir::TempDir;

    static WORK_DIR: Lazy<String> = Lazy::new(|| {
        let work_dir = TempDir::new("plrust-tests").expect("Couldn't create tempdir");
        format!("plrust.work_dir='{}'", work_dir.path().display())
    });

    pub fn setup(_options: Vec<&str>) {
        // perform one-off initialization when the pg_test framework starts
    }

    pub fn postgresql_conf_options() -> Vec<&'static str> {
        vec![&*WORK_DIR]
    }
}
