# What is PL/Rust?

> This documentation is under development.

PL/Rust is a loadable procedural language that enables writing PostgreSQL functions in the Rust programming
language. These functions are compiled to native machine code. Unlike other procedural languages, PL/Rust functions
are not interpreted.

The top advantages of PL/Rust include writing natively-compiled functions to achieve the absolute best performance,
access to Rust's large development ecosystem, and Rust's compile-time safety guarantees.

PL/Rust provides access to Postgres' Server Programming Interface (SPI) including dynamic queries, prepared
statements, and cursors. It also provides safe Rust types over most of Postgres built-in data types, including (but
not limited to), `TEXT`, `INT`, `BIGINT`, `NUMERIC`, `FLOAT`, `DOUBLE PRECISION`,
`DATE`, `TIME`, etc.

On `x86_64` and `aarch64` systems PL/Rust can be a "trusted" procedural language, assuming the proper compilation
requirements are met. On other systems, it is perfectly usable as an "untrusted" language but cannot provide the
same level of safety guarantees.

The following example shows an example PL/Rust function to count the length of
an input string.

```sql
CREATE FUNCTION strlen(name TEXT)
    RETURNS int LANGUAGE plrust AS
$$
    Ok(Some(name.unwrap().len() as i32))
$$;
```

Using the function is just like any other PostgreSQL function.

```sql
SELECT strlen('Hello, PL/Rust');
```

```bash
┌────────┐
│ strlen │
╞════════╡
│     14 │
└────────┘
```



