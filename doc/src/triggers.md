# Triggers

PL/Rust functions can be used to define trigger functions on data changes.
A trigger function is created using the `CREATE FUNCTION` command, declaring it as a function with no arguments and a return type of
`trigger`. Trigger variables are available from in `trigger`
to describe the condition that triggered the call and the `new` and `old`
rows.

PL/Rust trigger support options are [documented on docs.rs](https://docs.rs/pgx/latest/pgx/prelude/struct.PgTrigger.html) and defined in the `.rs` files in the
[trigger_support](https://github.com/tcdi/pgx/tree/master/pgx/src/trigger_support) directory.

These examples are an expansion of the code from [`plrust/plrust/src/tests.rs`](https://github.com/tcdi/plrust/blob/main/plrust/src/tests.rs). The elaborations here
illustrate additional functionality.

## Table for Triggers

Create the `plrust.dog` table to allow us to keep track of our dogs, and how much attention
they have received via a count of `scritches`.


```sql
CREATE TABLE plrust.dog (
    id BIGINT NOT NULL GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    name TEXT,
    scritches INT NOT NULL DEFAULT 0,
    last_scritch TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

The `name` column in `plrust.dog` is the only column without a default
value set.  The `scritches` and `last_scritch` column both have defaults set.
The goal of this design is to only have to define the `name` during `INSERT`.
Subsequent `UPDATE` queries should only have to update the
`last_scritch` column.

## Trigger example

The following example creates a trigger function named `plrust.dog_trigger()`.
The trigger will be used on `INSERT` and `UPDATE` with slightly different
behavior depending on which operation is being used. This logic is based
on the value of `trigger.op()?`, for `INSERT` the `trigger.new` object is used,
for `UPDATE` the `trigger.old` object is used.
This code is explained further after the code block.


```sql
CREATE FUNCTION plrust.dog_trigger()
RETURNS trigger AS
$$
    let tg_op = trigger.op()?;

    let my_row = match tg_op {
        INSERT => trigger.new().unwrap(),
        _ => trigger.old().unwrap()
    };
    let mut my_row = my_row.into_owned();

    let counter_field = "scritches";
    match my_row.get_by_name::<i32>(counter_field)? {
        Some(val) => my_row.set_by_name(counter_field, val + 1)?,
        None => (),
    }

    Ok(Some(my_row))
$$
LANGUAGE plrust;


CREATE TRIGGER dog_trigger
    BEFORE INSERT OR UPDATE ON plrust.dog
    FOR EACH ROW
    EXECUTE FUNCTION plrust.dog_trigger();
```

The `tg_op` variable is available from the `trigger.op()` method and has values
of `INSERT`, `UPDATE`, `DELETE` and `TRUNCATE`.  See the definition
of [`PgTriggerOperation` for more](https://docs.rs/pgx/latest/pgx/prelude/enum.PgTriggerOperation.html).
The `tg_op` value is used in a `match` to define the `my_row` variable.


```rust
let tg_op = trigger.op()?;

let my_row = match tg_op {
    INSERT => trigger.new().unwrap(),
    _ => trigger.old().unwrap()
};
let mut my_row = my_row.into_owned();
```

With the appropriate `my_row` identified, the next step is to increment the
`scritches` column by 1.  This is defined in the `counter_field` variable
for easy reuse. The `get_by_name` and `set_by_name` functions are used for
this operation.

```rust
let counter_field = "scritches";
match my_row.get_by_name::<i32>(counter_field)? {
    Some(val) => my_row.set_by_name(counter_field, val + 1)?,
    None => (),
}
```

Finally, the `my_row` is returned for the operation to proceed.

```rust
Ok(Some(my_row))
```


Next we `INSERT` a row and then query the table to observe the effects of the trigger.

```sql
INSERT INTO plrust.dog (name) VALUES ('Nami');
SELECT * FROM plrust.dog;
```

The results show that while the `DEFAULT` value for the `scritches` column is
defined as `0` in the table, the initial value is 1 because trigger updated
the value.


```bash
 id | name | scritches |         last_scritch          
----+------+-----------+-------------------------------
  1 | Nami |         1 | 2023-03-04 17:30:43.601525+00
```

If we update the record for Nami by setting the `last_scritch` value to `NOW()`
the trigger will increment the `scritches` column value for us.

```sql
UPDATE plrust.dog
    SET last_scritch = NOW()
    WHERE id = 1;

SELECT * FROM plrust.dog;
```


```
 id | name | scritches |         last_scritch          
----+------+-----------+-------------------------------
  1 | Nami |         2 | 2023-03-04 17:35:05.320482+00
```



## Not yet supported

Event Triggers and `DO` blocks are not (yet) supported by PL/Rust.


