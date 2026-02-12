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
    fn test_udt() -> spi::Result<()> {
        Spi::run(
            r#"
CREATE TYPE person AS (
    name text,
    age  float8
);

create function make_person(name text, age float8) returns person
    strict parallel safe
    language plrust as
$$
    // create the Heap Tuple representation of the SQL type `person`
    let mut p = PgHeapTuple::new_composite_type("person")?;

    // set a few of its attributes
    //
    // Runtime errors can occur if the attribute name is invalid or if the Rust type of the value
    // is not compatible with the backing SQL type for that attribute.  Hence the use of the `?` operator
    p.set_by_name("name", name)?;
    p.set_by_name("age", age)?;

    // return the `person`
    Ok(Some(p))
$$;

create function get_person_name(p person) returns text
    strict parallel safe
    language plrust as
$$
   // `p` is a `PgHeapTuple` over the underlying data for `person`
   Ok(p.get_by_name("name")?)
$$;

create function get_person_age(p person) returns float8
    strict parallel safe
    language plrust as
$$
   // `p` is a `PgHeapTuple` over the underlying data for `person`
   Ok(p.get_by_name("age")?)
$$;

create function get_person_attribute(p person, attname text) returns text
    strict parallel safe
    language plrust as
$$
   match attname.to_lowercase().as_str() {
    "age" => {
        let age:Option<f64> = p.get_by_name("age")?;
        Ok(age.map(|v| v.to_string()))
    },
    "name" => {
        Ok(p.get_by_name("name")?)
    },
    _ => panic!("unknown attribute: `{attname}`")
   }
$$;

create operator ->> (function = get_person_attribute, leftarg = person, rightarg = text);

create table people
(
    id serial8 not null primary key,
    p  person
);

insert into people (p) values (make_person('Johnny', 46.24));
insert into people (p) values (make_person('Joe', 99.09));
insert into people (p) values (make_person('Dr. Beverly Crusher of the Starship Enterprise', 32.0));
            "#,
        )?;

        let johnny = Spi::get_one::<PgHeapTuple<AllocatedByRust>>(
            "SELECT p FROM people WHERE p->>'name' = 'Johnny';",
        )?
        .expect("SPI result was null");

        let age = johnny.get_by_name::<f64>("age")?.expect("age was null");
        assert_eq!(age, 46.24);

        Ok(())
    }
}
