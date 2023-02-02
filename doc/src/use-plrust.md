# Creating functions with PL/Rust

These instructions explain how to create PostgreSQL functions using the Rust
language and `plrust`.


## Basic Example

The following example creates a simple `plrust` function named `one()`
that simply returns the integer `1`.

```sql
CREATE FUNCTION one() RETURNS INT LANGUAGE plrust AS $$ Ok(Some(1)) $$;
```

The ability to one-line the above example is nice. The following example is
the same function definition showing the PostgreSQL function structure more clearly.
The `CREATE FUNCTION` structure is outlined in the
[PostgreSQL documentation](https://www.postgresql.org/docs/current/sql-createfunction.html).
The final statement of the function's body returns `Ok(Some(<something>))`.

```sql
CREATE FUNCTION one()
    RETURNS INT
    LANGUAGE plrust
AS
$$
    Ok(Some(1))
$$
;
```


## Function with parameters

The next example creates a function named `strlen` that accepts a parameter
named `val`.
The function returns a `BIGINT` representing the character count of the text
in `val`.

```sql
CREATE FUNCTION strlen(val TEXT)
    RETURNS BIGINT
    LANGUAGE plrust
AS $$
    Ok(Some(val.unwrap().len() as i64))
$$;
```

The above example used `unwrap()` to parse the text variable.
Changing the function definition to include `STRICT` avoids the need to use
`unwrap()`.  The following example works the same as above.


```sql
CREATE FUNCTION strlen(val TEXT)
    RETURNS BIGINT
    LANGUAGE plrust STRICT
AS $$
    Ok(Some(val.len() as i64))
$$;
```


Using the `strlen()` function works as expected.

```sql
SELECT strlen('Hello, PL/Rust');
```

```
┌────────┐
│ strlen │
╞════════╡
│     14 │
└────────┘
```


## Calculations

Of course, `plrust` functions can performance calculations such as converting
distance values from feet to miles.

```sql
CREATE FUNCTION distance_feet_to_miles(feet FLOAT)
    RETURNS FLOAT
    LANGUAGE plrust STRICT
AS $$
    Ok(Some(feet / 5280.0))
$$;
```

Using the function.

```sql
SELECT distance_feet_to_miles(10000);
```

```
┌────────────────────────┐
│ distance_feet_to_miles │
╞════════════════════════╡
│      1.893939393939394 │
└────────────────────────┘
```


## Use dependencies

One of the powerful features of `plrust` is its ability to define `dependencies`
in the function.  The following examples use the
[`faker_rand` crate](https://docs.rs/faker_rand/latest/faker_rand/index.html)
in functions to generate fake text data.

The `random_first_name()` function returns a random first name using the
 `en_us` locale.
 

```sql
CREATE FUNCTION random_slogan() RETURNS TEXT
LANGUAGE plrust AS $$
[dependencies]
    faker_rand = "0.1"
    rand = "0.8"
[code]
    use faker_rand::en_us::company::Slogan;
    Ok(Some(rand::random::<Slogan>().to_string()))
$$;
```


```sql
SELECT random_slogan();
```

```
┌─────────────────────────────┐
│        random_slogan        │
╞═════════════════════════════╡
│ Ergonomic composite schemas │
└─────────────────────────────┘
```


```sql
CREATE FUNCTION random_company_name(locale TEXT)
    RETURNS TEXT
    LANGUAGE plrust STRICT
AS $$
[dependencies]
    faker_rand = "0.1"
    rand = "0.8"
[code]
    match locale {
        "en_us" => {
            use faker_rand::en_us::company::CompanyName;
            Ok(Some(rand::random::<CompanyName>().to_string()))
        }
        "fr_fr" => {
            use faker_rand::fr_fr::company::CompanyName;
            Ok(Some(rand::random::<CompanyName>().to_string()))
        }
        _ => panic!("Unsupported locale. Use en_us or fr_fr")
    }
$$;
```


```sql
SELECT random_company_name('en_us') AS en_us,
    random_company_name('fr_fr') AS fr_fr;
```


```
┌────────────┬───────────────┐
│   en_us    │     fr_fr     │
╞════════════╪═══════════════╡
│ Furman Inc │ Élisabeth SEM │
└────────────┴───────────────┘
```


