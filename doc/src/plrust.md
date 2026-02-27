# What is PL/Rust?

> This documentation is under development.

PL/Rust is a loadable procedural language that enables writing PostgreSQL
functions in the Rust programming language. These functions are compiled to
native machine code. Unlike other procedural languages, PL/Rust functions are
not interpreted.

The top advantages of PL/Rust include writing natively-compiled functions to achieve the absolute best performance,
access to Rust's large development ecosystem, and Rust's compile-time safety guarantees.

PL/Rust is Open Source and [actively developed on GitHub](https://github.com/tcdi/plrust).

## Features

PL/Rust provides access to Postgres' Server Programming Interface (SPI) including dynamic queries, prepared
statements, and cursors. It also provides safe Rust types over most of Postgres built-in data types, including (but
not limited to), `TEXT`, `INT`, `BIGINT`, `NUMERIC`, `FLOAT`, `DOUBLE PRECISION`,
`DATE`, `TIME`, etc.

On `x86_64` and `aarch64` systems PL/Rust can be a "trusted" procedural language, assuming the proper compilation
requirements are met. On other systems, it is perfectly usable as an "untrusted" language but cannot provide the
same level of safety guarantees.

## Example PL/Rust function

The following example shows an example PL/Rust function to count the length of
an input string. See [PL/Rust Functions and Arguments](./use-plrust.md)
for more examples.


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


## Built on pgrx

PL/Rust itself is a [`pgrx`](https://github.com/tcdi/pgrx)-based Postgres extension.  Furthermore, each `LANGUAGE
plrust` function are themselves mini-pgrx extensions. `pgrx`is a generalized framework for developing Postgres extensions with Rust.  Like this project, `pgrx`
is developed by [TCDI](https://www.tcdi.com).

The following sections discuss PL/Rusts safety guarantees, configuration settings, and installation instructions.

# General Safety, by Rust

Quoted from the "Rustonomicon":

> Safe Rust is the true Rust programming language. If all you do is write Safe Rust, you will never have to worry
> about type-safety or memory-safety. You will never endure a dangling pointer, a use-after-free, or any other kind
> of Undefined Behavior (a.k.a. UB).

This is the universe in which PL/Rust functions live. If a PL/Rust function compiles it has these guarantees, by
the Rust compiler, that it won't "crash." This quality is important for natively-compiled code running in a
production database.

## What about `unsafe`?

PL/Rust uses the Rust compiler itself to wholesale **disallow** the use of `unsafe` in user functions. If
a `LANGUAGE plrust` function uses `unsafe` it won't compile.

Generally, what this means is that PL/Rust functions cannot call `unsafe fn`s, cannot call `extern "C"`s into
Postgres itself, and cannot dereference pointers.

This is accomplished using Rust's built-in `#![forbid(unsafe_code)]` lint.

3rd-party crate dependencies are allowed to use `unsafe`. We'll discuss this below.

## What about `pgrx`?

If `pgrx` is a "generalized framework for developing Postgres extensions with Rust", and if PL/Rust user functions
are themselves "mini-pgrx extensions", what prevents a `LANGUAGE plrust` function from using any part of `pgrx`?

The [`plrust-trusted-pgrx`](https://github.com/tcdi/plrust/tree/main/plrust-trusted-pgrx) crate does!
The `plrust-trusted-pgrx` crate is a tightly-controlled "re-export crate" on top of `pgrx` that exposes the bare minimum necessary for
PL/Rust user functions to compile along with the bare minimum, **safe** features of `pgrx`.

The crate is versioned independently to both `pgrx` and `plrust` and is published on [crates.io](https://crates.io/crates/plrust-trusted-pgrx).
By default, the version a plrust user function will use is that of the one set in the project repository when plrust itself
is compiled.  However, the `plrust.trusted_pgrx_version` GUC can be set to specify a specific version.

The intent is that `plrust-trusted-pgrx` can evolve independently of both `pgrx` and `plrust`.

There are a few "unsafe" parts of `pgrx` exposed through `plrust-trusted-pgrx`, but PL/Rust's ability to block `unsafe`
renders them useless by PL/Rust user functions.  `plrust-trusted-pgrx`'s docs are available on [docs.rs](https://docs.rs/plrust-trusted-pgrx).


## What about Rust compiler bugs?

PL/Rust uses its own `rustc` driver which enables it to apply custom lints to the user's `LANGUAGE plrust` function.
In general, these lints will fail compilation if the user's code uses certain code idioms or patterns which we know to
have "I-Unsound" issues.

PL/Rust contains a small set of [lints](config-lints.md) to block what the developers have deemed the most egregious "I-Unsound" Rust bugs.

Should new Rust bugs be found, and detection lints are developed for PL/Rust, the lints can be applied to new user 
function compilations along with ensuring that future function executions had those lints applied at compile time.

Note that this is done on a best-effort basis, and does *not* provide a strong level of security — it's not a sandbox,
and as such, it's likely that a skilled hostile attacker who is sufficiently motivated could find ways around it
(PostgreSQL itself is not a particularly hardened codebase, after all). You should ensure such actors cannot execute SQL
on your database, but to be clear: this is true regardless of whether or not PL/Rust is installed. Having said that, any
issues found with our implementation will be taken seriously, and should be
[reported appropriately](https://github.com/tcdi/plrust/blob/main/SECURITY.md).


## Trusted with `postgrestd` on Linux x86_64/aarch64

The "trusted" version of PL/Rust uses a unique fork of Rust's `std` entitled
[`postgrestd`](https://github.com/tcdi/postgrestd) when compiling `LANGUAGE plrust` user functions. `postgrestd` is
a specialized Rust compilation target which disallows access to the filesystem and the host operating system. The Install PL/Rust section outlines the steps required for
[trusted install](/install-plrust.md#trusted-install) of PL/Rust.
Currently, `postgrestd` is only supported on Linux `x86_64` and `aarch64` platforms.

When `plrust` user functions are compiled and linked against `postgrestd`, they are prohibited from using the
filesystem, executing processes, and otherwise interacting with the host operating system.

In order for PL/Rust to use `postgrestd`, its Rust compilation targets must be installed on the Postgres server.
This happens via plrust's
[`plrust/build`](https://github.com/tcdi/plrust/blob/main/plrust/build) script, which clones `postgrestd`, compiles it, by
default, for both `x86_64` and `aarch64` architectures, and ultimately places a copy of the necessary libraries used by
Rust for `std` into the appropriate "sysroot", which is the location that `rustc` will look for building those
libraries.

## The `trusted` Feature Flag

PL/Rust has a feature flag simply named `trusted`. When compiled with the `trusted` feature flag PL/Rust will
**always** use the `postgrestd` targets when compiling user functions.
Again, this is only supported on `x86_64` and `aarch64` Linux systems.
`postgrestd` and the `trusted` feature flag are **not** supported on other platforms.
As such, PL/Rust cannot be considered fully trusted on those platforms.

If the `trusted` feature flag is not used when compiling PL/Rust, which is the default, then `postgrestd` is **not**
used when compiling user functions, and while they'll still benefit from Rust's general compile-time safety
checked, forced usage of the `plrust-trusted-pgrx` crate, and PL/Rust's `unsafe` blocking, they will be able to access the
filesystem and communicate with the host operating system, as the user running the connected Postgres backend
(typically, this is a user named `postgres`).


# PL/Rust is also a Cross Compiler

In this day and age of sophisticated and flexible Postgres replication, along with cloud providers offering
Postgres on, and replication to, disparate CPU architectures, it's important that plrust, since it stores the user
function binary bytes in a database table, support running that function on a replicated Postgres server of a
different CPU architecture.

*cross compilation has entered the chat*

By default, PL/Rust will not perform cross compilation. It must be installed
and enabled through configuration.

Configuring a *host* to properly cross compile is a thing that can take minimal effort to individual feats of
heroic effort. Reading the (still in-progress) [pgrx cross compile guide](https://github.com/tcdi/pgrx/blob/master/CROSS_COMPILE.md) 
can help. Generally speaking, it's not too awful to setup on Debian-based Linux systems, such as Ubuntu. Basically,
you install the "cross compilation toolchain" `apt` package for the *other* platform.

