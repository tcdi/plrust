# Logging to PostgreSQL from PL/Rust


```sql
CREATE FUNCTION one()
    RETURNS INT
    LANGUAGE plrust
AS
$$
    ereport!(PgLogLevel::LOG, PgSqlErrorCode::ERRCODE_SUCCESSFUL_COMPLETION, "A user ran the one() function");
    Ok(Some(1))
$$
;
```

Now whenever a user runs the `one()` function it will add a log entry
similar to the following.  The exact output depends on your PostgreSQL
logging configuration.

```
LOG:  A user ran the one() function
```


Using `PgLogLevel::INFO` instead of `PgLogLevel::LOG` will return the
notice back to the client.  In `psql` this might look like the following
example.

```bash
(localhost 🐘) plrust@plrust=# SELECT one();
INFO:  A user ran the one() function
┌─────┐
│ one │
╞═════╡
│   1 │
└─────┘
(1 row)

Time: 74.461 ms
```

