# `plrust` Extension for PostgreSQL

![CI status](https://github.com/zombodb/plrust/actions/workflows/ci.yml/badge.svg)

Support for `plrust` in PostgreSQL functions.

```SQL
CREATE EXTENSION IF NOT EXISTS plrust;
CREATE OR REPLACE FUNCTION sum_array(a BIGINT[]) RETURNS BIGINT
    IMMUTABLE STRICT
    LANGUAGE PLRUST AS
$$
[dependencies]
    # Add Cargo.toml dependencies here.
[code]
    Some(a.into_iter().map(|v| v.unwrap_or_default()).sum())
$$;
SELECT sum_array(ARRAY[1,2,3]);
/*
sum_array
----------------
              6
(1 row)
*/
```

# Options

There are two `postgresql.conf` settings that must be configured:

Option | Description
--------------|-----------
`plrust.pg_config` | The location of the `postgresql.conf`.
`plrust.work_dir` | The directory where pl/rust will build functions with cargo.
`plrust.tracing_level` | A [tracing directive][docs-rs-tracing-directive]. (Default `info`)

[github-pgx]: https://github.com/zombodb/pgx
[github-fpm]: https://github.com/jordansissel/fpm
[docs-rs-tracing-directive]: https://docs.rs/tracing-subscriber/0.3.11/tracing_subscriber/filter/struct.EnvFilter.html


# Installation

> **PL/Rust is a [`pgx`][github-pgx] extension.**
>
> If you're already familiar with [`pgx`][github-pgx] you'll find you already know how to hack on, package, and use PL/Rust.

First, ensure `cargo pgx` installed & initialized:

```bash
cargo install cargo-pgx
cargo pgx init --pg14 download
```

Then, add some configuration to the `postgresql.conf` and ensure there is a
writable `work_dir`:

```bash
PG_CONFIG=$(find ~/.pgx/14.*/pgx-install/bin/pg_config)
SCRATCH_DIR=/home/${USER}/git/zombodb/plrust/scratch
cat <<-EOF >> ~/.pgx/data-14/postgresql.conf
  plrust.pg_config = '${PG_CONFIG}'
  plrust.work_dir = '${SCRATCH_DIR}'
EOF
mkdir -p scratch
chmod -R 777 scratch
```

It's possible to test debug builds inside a working PostgreSQL:

```bash
$ cargo pgx run
    Stopping Postgres v12
building extension with features ``
"cargo" "build" "--no-default-features"
    Finished dev [unoptimized + debuginfo] target(s) in 0.56s

installing extension
     Copying control file to `target/release/plrust-pg14/usr/share/postgresql/14/extension/plrust.control`
     Copying shared library to `target/release/plrust-pg14/usr/lib/postgresql/14/lib/plrust.so`
     Writing extension schema to `target/release/plrust-pg12/usr/share/postgresql/14/extension/plrust--1.0.sql`
    Finished installing plrust
    Starting Postgres v14 on port 28812
    Re-using existing database plrust
psql (14.2)
Type "help" for help.

plrust=# \dx
                 List of installed extensions
  Name   | Version |   Schema   |         Description
---------+---------+------------+------------------------------
 plpgsql | 1.0     | pg_catalog | PL/pgSQL procedural language
(1 row)

plrust=# CREATE EXTENSION IF NOT EXISTS plrust;
CREATE EXTENSION
plrust=# \dx
                 List of installed extensions
  Name   | Version |   Schema   |         Description
---------+---------+------------+------------------------------
 plpgsql | 1.0     | pg_catalog | PL/pgSQL procedural language
 plrust  | 1.0     | plrust     | plrust:  Created by pgx
(2 rows)
```

To install it to the locally running PostgreSQL server:

```bash
cargo pgx install
```

## Creating a distributable

```bash
cargo pgx package
```

The output should resemble the following:

```bash
$ cargo pgx package
building extension with features ``
"cargo" "build" "--release" "--message-format=json-render-diagnostics"
   Compiling plrust v0.0.0 (/home/${USER}/git/zombodb/plrust)
    Finished release [optimized] target(s) in 28.37s

installing extension
     Copying control file to `target/release/plrust-pg14/usr/share/postgresql/14/extension/plrust.control`
     Copying shared library to `target/release/plrust-pg14/usr/lib/postgresql/14/lib/plrust.so`
     Writing extension schema to `target/release/plrust-pg12/usr/share/postgresql/14/extension/plrust--1.0.sql`
    Finished installing plrust
```

The directory tree inside `target/release/plrust-pg14/` contains a tree corresponding to the local 
where the extension should be placed.

From here, you can use an archive or a tool like [`fpm`][github-fpm] to distribute the artifact to
the target.

Once installed in the right directory, connect via `psql`:

```sql
psql=# CREATE EXTENSION IF NOT EXISTS plrust;
CREATE EXTENSION
plrust=# \dx
                 List of installed extensions
  Name   | Version |   Schema   |         Description          
---------+---------+------------+------------------------------
 plpgsql | 1.0     | pg_catalog | PL/pgSQL procedural language
 plrust  | 1.0     | plrust     | plrust:  Created by pgx
(2 rows)
plrust=# \dx+ plrust
      Objects in extension "plrust"
           Object description            
-----------------------------------------
 function plrust.plrust_call_handler()
 function plrust.plrust_validator(oid)
 function plrust.recompile_function(oid)
 language plrust
(4 rows)
```