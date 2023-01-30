# PL/Rust: A Trusted Procedural Language Handler

PL/Rust is a loadable procedural language that enables writing PostgreSQL functions in the Rust programming
language. These functions are compiled to native machine code. Unlike other procedural languages, PL/Rust functions
are not interpreted.

The top advantages of PL/Rust include writing natively-compiled functions to achieve the absolute best performance,
access to Rust's large development ecosystem, and Rust's compile-time safety guarantees.

PL/Rust provides access to Postgres' Server Programming Interface (SPI) including dynamic queries, prepared
statements, and cursors. It also provides safe Rust types over most of Postgres built-in data types, including (but
not limited to), TEXT, INT, BIGINT, NUMERIC, FLOAT, DOUBLE PRECISION, DATE, TIME, etc.

On x86_64 and aarch64 systems PL/Rust can be a "trusted" procedural language, assuming the proper compilation
requirements are met. On other systems, it is perfectly usable as an "untrusted" language but cannot provide the
same level of safety guarantees.

An example PL/Rust function:

```sql
// return the character length of a text string
CREATE FUNCTION strlen(name TEXT) RETURNS int LANGUAGE plrust AS $$
    Ok(Some(name?.len() as i32))
$$;

# select strlen('Hello, PL/Rust');
strlen 
--------
     14
```

PL/Rust itself is a [`pgx`](https://github.com/tcdi/pgx)-based Postgres extension.  Furthermore, each `LANGUAGE
plrust` function are themselves mini-pgx extensions. `pgx`is a generalized framework for developing Postgres extensions with Rust.  Like this project, `pgx`
is developed by [TCDI](https://www.tcdi.com).

The following sections discuss PL/Rusts safety guarantees, configuration settings, and installaiton instructions.

# General Safety, by Rust

Quoted from the "Rustonomicon":

> Safe Rust is the true Rust programming language. If all you do is write Safe Rust, you will never have to worry
> about type-safety or memory-safety. You will never endure a dangling pointer, a use-after-free, or any other kind
> of Undefined Behavior (a.k.a. UB).

This is the universe in which PL/Rust functions live. If a PL/Rust function compiles it has these guarantees, by
the Rust compiler, that it won't "crash." This quality is important for natively-compiled code running in a
production database.

## What about `unsafe`?

PL/Rust uses the Rust compiler itself to wholesale **disallow** the use of `unsafe{}` blocks in user functions. If
a `LANGUAGE plrust` function contains such a block, it won't compile.

Generally, what this means is that PL/Rust functions cannot call `unsafe fn`s, cannot declare `extern "C"`s into
Postgres itself, and cannot dereference pointers.

3rd-party crate dependencies are allowed to use `unsafe`. We'll discuss this below.

## What about `pgx`?

If `pgx` is a "generalized framework for developing Postgres extensions with Rust", and if PL/Rust user functions
are themselves "mini-pgx extensions", what prevents a `LANGUAGE plrust` function from using any part of `pgx`?

The [`trusted-pgx`](https://github.com/tcdi/plrust/tree/main/trusted-pgx) crate does!

`trusted-pgx` is a tightly-controlled "re-export crate" on top of `pgx` that exposes the bare minimum necessary for
PL/Rust user functions to compile along with the bare minimum, **safe** features of `pgx`.

There are a few "unsafe" parts of `pgx` exposed through `trusted-pgx`, but PL/Rust's ability to block `unsafe`
renders them useless by PL/Rust user functions.

## Trusted with `postgrestd` on Linux x86_64/aarch64

The "trusted" version of PL/Rust uses a unique fork of Rust's `std` entitled
[`postgrestd`](https://github.com/tcdi/postgrestd) when compiling `LANGUAGE plrust` user functions. `postgrestd` is
a specialized Rust compilation target which disallows access to the filesystem and the host operating system.

Currently, `postgrestd` is only supported on Linux x86_64 and aarch64 platforms.

When `plrust` user functions are compiled and linked against `postgrestd`, they are prohibited from using the
filesystem, executing processes, and otherwise interacting with the host operating system.

In order for PL/Rust to use `postgrestd`, its Rust compilation targets must be installed on the Postgres server.
This happens via plrust's [`plrust/build`](plrust/build) script, which clones `postgrestd`, compiles it, by
default, for both x86_64 and aarch64 architectures, and ultimately places a copy of the necessary libraries used by
Rust for `std` into the appropriate "sysroot", which is the location that rustc will look for building those
libraries.

## The `trusted` Feature Flag

PL/Rust has a feature flag simply named `trusted`. When compiled with the `trusted` feature flag PL/Rust will
**always** use the `postgrestd` targets when compiling user functions. Again, this is only supported on x86_64 and
aarch64 Linux systems.

`postgrestd` and the `trusted` feature flag are **not** supported on other platforms. As such, PL/Rust cannot be
considered fully trusted on those platforms.

If the `trusted` feature flag is not used when comiling PL/Rust, which is the default, then `postgrestd` is **not**
used when compiling user functions, and while they'll still benefit from Rust's general compile-time safety
checked, forced usage of the `trusted-pgx` crate, and PL/Rust's `unsafe` blocking, they will be able to access the
filesystem and communicate with the host operating system, as the user running the connected Postgres backend
(typically, this is a user named `postgres`).

# PL/Rust is also a Cross Compiler

In this day and age of sophisticated and flexible Postgres replication, along with cloud providers offering
Postgres on, and replication to, disparate CPU architectures, it's important that plrust, since it stores the user
function binary bytes in a database table, support running that function on a replicated Postgres server of a
different CPU architecture.

*cross compilation has entered the chat*

By default, plrust will not perform cross compilation. It must be turned on through configuration.

Configuring a *host* to properly cross compile is a thing that can take minimal effort to individual feats of
heroic effort. Reading the (still in-progress) guide at https://github.com/tcdi/pgx/blob/master/CROSS_COMPILE.md
can help. Generally speaking, it's not too awful to setup on Debian-based Linux systems, such as Ubuntu. Basically,
you install the "cross compilation toolchain" `apt` package for the *other* platform.

For full "trusted" PL/Rust user functions, `postgrestd` is required and must also be installed.

# Installing PL/Rust

Installing PL/Rust and especially `postgrestd` requires a normal installation of Rust via
[`rustup`](https://rustup.rs) and for the relevant locations to be writeable on the building host.

These steps assume cross compilation is also going to be used. If not, simply remove references to the architecture
that isn't yours.

## Install `cargo-pgx`

PL/Rust is a [`pgx`](https://github.com/tcdi/pgx)-based Postgres extension and requires it be installed.

```bash
$ cargo install cargo-pgx --version 0.7.0 --locked
$ cargo pgx init
```

Next, lets clone this repo:

```bash
$ git clone https://github.com/tcdi/plrust.git
$ cd plrust
```

## Cross Compilation Support

If you want cross-compilation support, install the Rust targets for aarch64 and x86_64, then install `postgrestd`.
These are necessary to cross compile `postgrestd` and PL/Rust user functions.

```bash
$ cd plrust
$ rustup target install aarch64-unknown-linux-gnu
$ rustup target install x86_64-unknown-linux-gnu
```

Once finished, while still in the plrust directory subdirectory, run the `postgrestd` build script. This
example assumes that the `pg_config` binary from Postgres v15 is on your $PATH. If v15 is not your intended
Postgres version, change it to the proper major version number.

```bash
$ PG_VER=15 \
STD_TARGETS="x86_64-postgres-linux-gnu aarch64-postgres-linux-gnu" \
./build
```

(note: the above environment variables are the default... you can just run `./build`)

This will take a bit of time as it clones the `postgrestd` repository, builds it for two architectures, and finally
runs PL/Rust's entire test suite in "trusted" mode.

## Install PL/Rust

Installing the `plrust` extension is simple. Make sure the `pg_config` binary for the Postgres installation on the
host is in the `$PATH`, and simply run:

```bash
$ cargo pgx install --release --features "trusted"
```

Alternatively, you can specify the path to `pg_config`:

```bash
$ cargo pgx install --release --features "trusted" -c /path/to/pg_config
```

If you'd prefer PL/Rust be "untrusted" and haven't also installed `postgrestd` for at least the host architecture,
you can omit the `--features "trusted"` arguments.

# Configuration

PL/Rust has a few required configuration option, but first and foremost it **must** be configured as a
`shared_preload_libraries` entry in `postgresql.conf`. For example:

```
shared_preload_libraries = 'plrust'
```

Failure to do so will cause the plrust extension to raise an ERROR whenever Postgres tries to first load it.

The other available configuration, some of which are **required** are:

| Option                             | Type   | Description                                                        | Required | Default                                                    |
|------------------------------------| ------ |--------------------------------------------------------------------|----------|------------------------------------------------------------|
| `plrust.work_dir`                  | string | The directory where pl/rust will build functions with cargo        | yes      | <none>                                                     |
| `plrust.PATH_override`             | string | If `cargo` and `cc` aren't in the `postmaster`'s `$PATH`, set this | no       | environment or `~/.cargo/bin:/usr/bin` if `$PATH` is unset |
| `plrust.tracing_level`             | string | A [tracing directive][docs-rs-tracing-directive]                   | no       | `'info'`                                                   |
| `plrust.compilation_targets`       | string | Comma separated list of CPU targets (x86_64, aarch64)              | no       | <none>                                                     |
| `plrust.x86_64_linker`             | string | Name of the linker `rustc` should use on fo cross-compile          | no       | `'x86_64_linux_gnu_gcc'`                                   |
| `plrust.aarch64_linker`            | string | Name of the linker `rustc` should use on for cross-compile         | no       | `'aarch64_linux_gnu_gcc'`                                  |
| `plrust.x86_64_pgx_bindings_path`  | string | Path to output from `cargo pgx cross pgx-target` on x86_64         | no-ish   | <none>                                                     |
| `plrust.aarch64_pgx_bindings_path` | string | Path to output form `cargo pgx cross pgx-target` on aarch64        | no-ish   | <none>                                                     |

For PL/Rust to cross compile user functions it needs to know which CPU architectures via
`plrust.compilation_targets`. This is a comma-separated list of values, of which only `x86_64` and `aarch64` are
currently supported.

The architecture linker names have sane defaults and shouldn't need to be be changed (unless the host is some
esoteric Linux distro we haven't encountered yet).

The `plrust.{arch}_pgx_bindings_path` settings are actually required but PL/Rust will happily cross compile without them. If unspecified,
PL/Rust will use the pgx bindings of the host architecture for the cross compilation target architecture too. In other words, if the host 
is `x86_64` and PL/Rust is configured to cross compile to `aarch64` and the `plrust.aarch64_pgx_bindings_path` is *not* configured, it'll
blindly use the bindings it already has for `x86_64`.  This may or may not actually work.

To get the bindings, install `cargo-pgx` on the other system and run `cargo pgx cross pgx-target`. That'll generate a tarball. Copy that back 
to the primary host machine and untar it somewhere (plrust doesn't care where), and use that path as the configuration setting.


Note that it is perfectly fine (and really, expected) to set all of these configuration settings on both architectures.
plrust will silently ignore the one for the current host.  In other words, plrust only uses them when cross compiling for 
the other architecture.


# Quickly Getting Started

To quickly get started using PL/Rust for evaluation purposes, install `cargo-pgx` following the steps from above, then...

```bash
$ git clone https://github.com/tcdi/plrust.git
$ cd plrust/plrust
$ cargo pgx run pg14
psql> \q

$ SCRATCH_DIR=/home/${USER}/plrust-scratch
$ cat <<-EOF >> ~/.pgx/data-14/postgresql.conf
  plrust.work_dir = '${SCRATCH_DIR}'
EOF
$ mkdir -p scratch
$ chmod -R 777 scratch
```

Then run it for real and start writing functions!

```bash
$ cargo pgx run pg14
psql> CREATE EXTENSION plrust;
psql> CREATE FUNCTION strlen(name TEXT) RETURNS int LANGUAGE plrust AS $$
    Some(name?.len() as i32)
$$;
psql> select strlen('Hello, PL/Rust');
strlen 
--------
     14
```


# Other Notes

In the Postgres world it seems common for procedural languages to have two styles, "trusted" and "untrusted".  The consensus is to name those as "lang" and "langu", respectively -- where the "u" is supposed to represent "untrusted" (see "plperl" v/s "plperlu" for example).

plrust does not do this.  The only thing that Postgres uses to determine if a language handler is considered "trusted" is if it was created using `CREATE TRUSTED LANGUAGE`.  It does not inspect the name.

plrust stores the compiled user function binaries as a `bytea` in an extension-specific table uniquely key'd with its compilation target.

As such, compiling a function with an "untrusted" version of plrust, then installing the "trusted" version and trying to run that function will fail -- "trusted" and "untrusted" are considered different compilation targets and are not compatible with each other, even if the underlying hardware is exactly the same.

This does mean that it is not possible to install both "trusted" and "untrusted" versions of plrust on the same Postgres database cluster.

In the future, as `postgrestd` is ported to more platforms, we will seriously consider having both `plrust` and `plrustu`.  Right now, since "trusted" is only possible on Linux x86_64/aarch64, our objective is to drive production installations to be "trusted", while allowing non-Linux developers the ability to use `LANGUAGE plrust` too.


# Security Notice

Please read the [Security](SECURITY.md) for directions on reporting a potential security issue.

# License

PL/Rust is licensed under "The PostgreSQL License", which can be found [here](LICENSE.md).

[docs-rs-tracing-directive]: https://docs.rs/tracing-subscriber/0.3.11/tracing_subscriber/filter/struct.EnvFilter.html
