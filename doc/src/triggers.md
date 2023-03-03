# Triggers


https://github.com/tcdi/pgx/tree/master/pgx/src/trigger_support


https://github.com/tcdi/pgx/blob/master/pgx/src/trigger_support/pg_trigger_safe.rs


https://docs.rs/pgx/latest/pgx/prelude/enum.PgTriggerOperation.html



> NOTE:  `current` is what Postgres users will recognize as `OLD`, see https://github.com/tcdi/plrust/issues/209


Example based on `plrust/plrust/src/tests.rs`.

```sql
CREATE TABLE plrust.dogs (
    name TEXT,
    scritches INT NOT NULL DEFAULT 0,
    last_scritch TIMESTAMPTZ NULL
);


/*
    WARNING - This is a hot mess from my testing!  Need to split apart ideas
    into separate examples and move some things forward.
*/
CREATE FUNCTION plrust.pet_trigger() RETURNS trigger AS $$
    let current = trigger.current().unwrap();
    let mut current = current.into_owned();
    let new = trigger.new().unwrap();
    let mut new = new.into_owned();

    let field = "scritches";
    let update_field = "last_scritch";

    let current_counter = current.get_by_name::<i32>(field)?.unwrap();
    let new_counter = new.get_by_name::<i32>(field)?.unwrap();

    ereport!(PgLogLevel::INFO,
        PgSqlErrorCode::ERRCODE_SUCCESSFUL_COMPLETION,
        format!("Testing {current_counter} - {new_counter}"));

    match current.get_by_name::<i32>(field).unwrap() {
        Some(val) => current.set_by_name(field, val + 1).unwrap(),
        None => (),
    }

    Ok(current)
$$ LANGUAGE plrust;


CREATE TRIGGER pet_trigger BEFORE INSERT OR UPDATE ON plrust.dogs
    FOR EACH ROW EXECUTE FUNCTION plrust.pet_trigger();

INSERT INTO plrust.dogs (name) VALUES ('Nami');


SELECT * FROM plrust.dogs;
```

## Not yet supported

Event Triggers and `DO` blocks aren't (yet) supported.




## QUESTIONS

What's the best rust way to handle when a value might not be set?  Is it using match w/ `Some()` and `None()`?

e.g. this fails if the `field` isn't included in the insert.
```rust
let new_counter = new.get_by_name::<i32>(field)?.unwrap();
```

error

```
ERROR:  called `Option::unwrap()` on a `None` value
```


----

Attempting to pull other details from `trigger` might or might not work the same
right now.  See https://github.com/tcdi/plrust/issues/210.

