# PL/Rust: A Trusted Procedural Language Handler 

PL/Rust is a loadable procedural language that enables you to write PostgreSQL functions  in the Rust programming
language. These functions are compiled to native machine code (ie, not interpreted like PL/Pgsql or PL/Perl).

The top advantages of PL/Rust include writing natively-compiled functions to achieve the absolute best performance, 
access to Rust's large development ecosystem, and Rust's compile-time safety guarantees.

On x86_64 and aarch64 systems, PL/Rust can be considered a fully trusted procedural language, assuming the proper
compilation and host system requirements are met.  On other systems, it is perfectly usable as an "untrusted" language
but cannot provide the same level of safety guarantees.

This is an example PL/Rust function:

```sql
CREATE FUNCTION strlen(name TEXT) RETURNS int LANGUAGE plrust AS $$
    Some(name?.len() as i32)
$$;

# select strlen('Hello, PL/Rust');
strlen 
--------
     14
```

# General Safety, by Rust



# Trusted-ness with `postgrestd`

PL/Rust uses a unique fork of Rust's `std` entitled [`postgrestd`](https://github.com/tcdi/postgrestd) when compiling 
user `LANGUAGE plrust` functions. `postgrestd` allows PL/Rust to be a "Trusted Procedural Language" on platforms where 
it's supported.  Currently, this is only x86_64 and aarch64 Linux systems.

When `plrust` user functions are compiled and linked against `postgrestd`, PL/Rust prohibits them from using the 
filesystem, executing processes, and otherwise interacting with the host operating system.  All in an effort to meet 
Postgres' "Trusted Procedural Language" requirements.

In order for PL/Rust to use `postgrestd`, its targets must be installed on the Postgres server.  This happens via
the [`build`](plrust/build) script, which clones `postgrestd` from https://github.com/tcdi/postgrestd, compiles it,
by default, for both x86_64 and aarch64 architectures, and ultimately places a copy of the necessary libraries used by 
Rust for `std` into the appropriate "sysroot", which is the location that rustc will look for building those libraries.

Additionally, PL/Rust itself must be compiled with the `trusted` feature flag.  More on this below.

When compiled with the `trusted` feature flag PL/Rust will **always** use the `${arch}-postgres-linux-gnu` rust targets 
on either x86_64 or aarch64 Linux systems.  This can be turned off.

`postgrestd` and the `trusted` feature flag are **not** supported on other platforms.  As such, PL/Rust cannot be 
considered fully trusted on those platforms.

## TODO:  PL/Rust is also a Cross Compiler
...

## Installing `postgrestd`

This initial build process requires a normal installation of Rust via [`rustup`](https://rustup.rs)
and for the relevant location to be writeable on the building host.

```bash
cd plrust
rustup target install aarch64-unknown-linux-gnu
rustup target install x86_64-unknown-linux-gnu
./build
cargo build
```

# Configuration

First, plrust must be configured as a `shared_preload_libraries` entry in `postgresql.conf`.  For example:

```
shared_preload_libraries = 'plrust'
```

Failure to do so will cause unexpected behavior around the DROP FUNCTION, DROP SCHEMA, and ALTER FUNCTION commands.

Additionally, there are two `postgresql.conf` settings that must be configured:

| Option                  | Type    | Description                                                 | Required | Default  |
|-------------------------|---------|-------------------------------------------------------------|----------|----------|
| `plrust.pg_config`      | string  | The full path of the `pg_config` binary                     | yes      | <none>   |
| `plrust.work_dir`       | string  | The directory where pl/rust will build functions with cargo | yes      | <none>   |
| `plrust.tracing_level`  | string  | A [tracing directive][docs-rs-tracing-directive]            | no       | `'info'` |

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

<!-- If `cargo expand` (a very useful tool for debugging pgx-macros) is used to the plrust crate,
    it embeds the README.md in a block doc comment: /** */. To preserve correct Rust highlighting,
    balance the upcoming bash glob with a comment-open: /* -->
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
 language plrust
 table plrust.plrust_proc
(4 rows)
```

## Security Notice

Please read the [Security](SECURITY.md) for directions on reporting a potential security issue.
