# `plrust` Extension for PostgreSQL

![CI status](https://github.com/zombodb/plrust/actions/workflows/ci.yml/badge.svg)

# 🚨 NOTICE 🚨 

This repo has relocated from `https://github.com/zombodb/pgx` to this location (`https://github.com/tcdi/pgx`).  You may need to update your remote in `.git/config` to reflect this change.

---

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

## Installation

Ensure `cargo pgx` installed & initialized:

```bash
cargo install cargo-pgx
cargo pgx init
```

Then, add some configuration to the `postgresql.conf` and ensure there is a
writable `work_dir`:

```bash
cat <<-EOF >> ~/.pgx/data-14/postgresql.conf
  plrust.pg_config = '/home/${USER}/.pgx/14.2/pgx-install/bin/pg_config'
  plrust.work_dir = '/home/${USER}/git/zombodb/plrust/scratch'
EOF
mkdir -p scratch
chmod -R 777 scratch
```

It's possible to test debug builds inside a working PostgreSQL with:

```bash
cargo pgx run pg12
```

The output should resemble:

```bash
$ cargo pgx run pg12
    Stopping Postgres v12
building extension with features `pg12`
"cargo" "build" "--features" "pg12" "--no-default-features"
    Finished dev [unoptimized + debuginfo] target(s) in 0.56s

installing extension
     Copying control file to `/home/nixos/.pgx/12.6/pgx-install/share/postgresql/extension/plrust.control`
     Copying shared library to `/home/nixos/.pgx/12.6/pgx-install/lib/postgresql/plrust.so`
     Writing extension schema to `/home/nixos/.pgx/12.6/pgx-install/share/postgresql/extension/plrust--1.0.sql`
    Finished installing plrust
    Starting Postgres v12 on port 28812
    Re-using existing database plrust
psql (12.6)
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
building extension with features `pg12`
"cargo" "build" "--release" "--features" "pg12" "--no-default-features"
   Compiling pgx-pg-sys v0.1.19 (/git/zombodb/pgx/pgx-pg-sys)
   Compiling pgx v0.1.19 (/git/zombodb/pgx/pgx)
   Compiling plrust v0.0.0 (/git/zombodb/plrust)
    Finished release [optimized] target(s) in 52.76s

installing extension
     Copying control file to `target/release/plrust-pg12/usr/share/postgresql/12/extension/plrust.control`
     Copying shared library to `target/release/plrust-pg12/usr/lib/postgresql/12/lib/plrust.so`
     Writing extension schema to `target/release/plrust-pg12/usr/share/postgresql/12/extension/plrust--1.0.sql`
    Finished installing plrust
```

The directory tree inside `target/release/plrust-pg12/` contains a tree corresponding to the local 
where the extension should be placed.

From here, you can use an archive or a tool like [`fpm`][github-fpm] to distribute the artifact to
the target.

Once installed in the right directory, connect via `psql`:

```bash
psql=# CREATE EXTENSION IF NOT EXISTS plrust;
CREATE EXTENSION
psql=# \dx
                 List of installed extensions
  Name   | Version |   Schema   |         Description          
---------+---------+------------+------------------------------
 plpgsql | 1.0     | pg_catalog | PL/pgSQL procedural language
 plrust  | 1.0     | plrust     | plrust:  Created by pgx
(2 rows)
```

# Options

There are two `postgresql.conf` settings that must be configured:

Option | Description
--------------|-----------
`plrust.pg_config` | The location of the `postgresql.conf`.
`plrust.work_dir` | The directory where pl/rust will build functions with cargo.

[github-pgx]: https://github.com/zombodb/pgx
[github-fpm]: https://github.com/jordansissel/fpm
