# Creating functions with PL/Rust

These instructions explain how to create PostgreSQL functions using the Rust
language and `plrust`.

Includes

* What you can use in `plrust`
* What you cannot use


## Basic Example

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

As `postgres` linux user


```sql
CREATE FUNCTION strlen(name TEXT)
    RETURNS BIGINT
    LANGUAGE plrust
AS $$
    Ok(Some(name.unwrap().len() as i64))
$$;
```

This:

```sql
CREATE FUNCTION dist_ft_to_mi(feet NUMERIC)
    RETURNS NUMERIC
    LANGUAGE plrust
AS $$
    Ok(Some(feet.unwrap() / 5280.0 as f64))
$$;

```

Fails with:

```
   error[E0369]: cannot divide `Option<trusted_pgx::AnyNumeric>` by `f64`
    --> src/lib.rs:9:22
     |
   9 |         Ok(Some(feet / 5280.0 as f64))
     |                 ---- ^ ------------- f64\
```


However, this works.

```sql
CREATE FUNCTION dist_ft_to_mi(feet FLOAT)
    RETURNS FLOAT
    LANGUAGE plrust
AS $$
    Ok(Some(feet.unwrap() / 5280.0 as f64))
$$;
```