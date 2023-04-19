# Logging to PostgreSQL from PL/Rust

PL/Rust provides the ability to log details using PostgreSQL's logging
system.  This functionality is exposed from pgrx via
[plrust/plrust-trusted-pgrx/src/lib.rs](https://github.com/tcdi/plrust/blob/main/plrust-trusted-pgrx/src/lib.rs).

The macros available for logging are defined:

```rust
pub use ::pgrx::{
    debug1, debug2, debug3, debug4, debug5, ereport, error, info, log, notice, warning,
};
```

## Basic logging

Using the `log!()` macro will send the message defined in the function to the
PostgreSQL logs defined by your `postgresql.conf`.  Running the following
example of `plrust.one()` creates a `LOG` record.

```sql
CREATE OR REPLACE FUNCTION plrust.one()
    RETURNS INT
    LANGUAGE plrust
AS
$$
    log!("Hello!  Friendly log message here.");
    Ok(Some(1))
$$
;
```

Running `plrust.one()` will run normally and the client running the query will
be presented with the results.  The `log!()` macro adds the defined log
message to the PostgreSQL log file.

The exact contents on the log line created in PostgreSQL's log file depends
on your `postgresql.conf` settings related to logging.  The following example
is what it may look like. 

```bash
2023-03-04 16:06:40 UTC [8109]: [15-1] user=postgres,db=plrust,app=psql,client=[local],query_id=-2211430114177040240  LOG:  Hello!  Friendly log message here.
```

The remainder of logging examples will only show the details controlled by PL/Rust
like the following example.

```bash
LOG:  Hello!  Friendly log message here.
```

Logging is not limited to static messages.  Values from the function can be included
using the `{variable}` notation.  Beware of data types, the `i32` value returned
by the `plrust.one()` function needs to be converted `.to_string()` to include
in the logged message string.


```sql
CREATE OR REPLACE FUNCTION plrust.one()
    RETURNS INT
    LANGUAGE plrust
AS
$$
    let one_val = 1_i32;
    log!("The plrust.one() function is returning: {one_val}");
    Ok(Some(one_val))
$$
;
```

When the above function runs, the resulting log line looks like the following.

```
LOG:  The plrust.one() function is returning: 1
```



## Warnings

Use the `warning!()` macro to log a more severe message.
Warnings are sent to the log file as well as being returned to the client as a
`WARNING`.


```sql
CREATE OR REPLACE FUNCTION plrust.one()
    RETURNS INT
    LANGUAGE plrust
AS
$$
    let expected_val = 1_i32;
    let one_val = 2_i32;

    if expected_val != one_val {
        warning!("The value for plrust.one() is unexpected. Found {one_val}")
    } else {
        log!("The plrust.one() function is returning: {one_val}");
    }

    Ok(Some(one_val))
$$
;
```

The following `WARNING` message is sent to the PostgreSQL log and to the client.

```bash
WARNING:  The value for plrust.one() is unexpected. Found 2
```

Running the above in `psql` looks like the following example.
You can see the user is presented with the warning message as well as the results
showing the `one` function returning the value `2`.


```bash
plrust=# select plrust.one();
WARNING:  The value for plrust.one() is unexpected. Found 2
DETAIL:  
 one 
-----
   2
(1 row)
```


## Errors

There are cases when a function simply cannot proceed and these errors need to
be logged.  The following example changes the `warning` from the previous example
to an `error`. 


```sql
CREATE OR REPLACE FUNCTION plrust.one()
    RETURNS INT
    LANGUAGE plrust
AS
$$
    let expected_val = 1_i32;
    let one_val = 2_i32;
    let one_val_str = one_val.to_string();

    if expected_val != one_val {
        error!("Invalid for plrust.one(). Found {one_val_str}")
    } else {
        log!("The plrust.one() function is returning: {one_val_str}");
    }

    Ok(Some(one_val))
$$
;
```

When PL/Rust runs the `error!()` macro the message is logged to the log file,
returned to the client, and the execution of the function is terminated with a panic.
In `psql` the user sees:

```bash
plrust=# select plrust.one();
ERROR:  Invalid for plrust.one(). Found 2
DETAIL:  
plrust=# 
```

In the PostgreSQL logs the following output is recorded.  Notice the `panicked`
line prior to the `ERROR` reported by the PL/Rust function.


```bash
thread '<unnamed>' panicked at 'Box<dyn Any>', /var/lib/postgresql/.cargo/registry/src/github.com-1ecc6299db9ec823/pgrx-pg-sys-0.7.2/src/submodules/panic.rs:160:13
ERROR:  Invalid for plrust.one(). Found 2
```



## Notifying the user

Using `notice!()` and `info!()` macros return the message to the client running
the query, allowing functions to provide feedback to the user running
the query.  These options do not log the message to the PostgreSQL logs.


```sql
CREATE OR REPLACE FUNCTION plrust.one()
    RETURNS INT
    LANGUAGE plrust
AS
$$
    notice!("Hello, this is a notice");
    Ok(Some(1))
$$
;
```

Running `SELECT plrust.one()` returns the expected value of `1`
along with the defined notice. Using `psql` returns and example
like the following code block.

```bash
NOTICE:  Hello, this is a notice
DETAIL:  
┌─────┐
│ one │
╞═════╡
│   1 │
└─────┘
```


## Using ereport

For the most control over logging you can use the `ereport!()` macro.
This is not necessary for most use cases.


```sql
CREATE FUNCTION one()
    RETURNS INT
    LANGUAGE plrust
AS
$$
    ereport!(PgLogLevel::LOG,
        PgSqlErrorCode::ERRCODE_SUCCESSFUL_COMPLETION,
        "A user ran the one() function");
    Ok(Some(1))
$$
;
```

