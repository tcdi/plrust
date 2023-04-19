# PL/Rust: A Trusted Procedural Language Handler

PL/Rust is a loadable procedural language that enables writing PostgreSQL functions in the Rust programming
language. These functions are compiled to native machine code. Unlike other procedural languages, PL/Rust functions
are not interpreted.

The top advantages of PL/Rust include writing natively-compiled functions to achieve the absolute best performance,
access to Rust's large development ecosystem, and Rust's compile-time safety guarantees.

> The PL/Rust [documentation is moving](https://tcdi.github.io/plrust/) to a more user friendly format.  The mdbook format documentation is auto-generated from the main branch.

PL/Rust provides access to Postgres' Server Programming Interface (SPI) including dynamic queries, prepared
statements, and cursors. It also provides safe Rust types over most of Postgres built-in data types, including (but
not limited to), TEXT, INT, BIGINT, NUMERIC, FLOAT, DOUBLE PRECISION, DATE, TIME, etc.

On x86_64 and aarch64 systems PL/Rust can be a "trusted" procedural language, assuming the proper compilation
requirements are met. On other systems, it is perfectly usable as an "untrusted" language but cannot provide the
same level of safety guarantees.

An example PL/Rust function:

```sql
-- return the character length of a text string
CREATE FUNCTION strlen(name TEXT) RETURNS int LANGUAGE plrust AS $$
    Ok(Some(name.unwrap().len() as i32))
$$;

# select strlen('Hello, PL/Rust');
strlen 
--------
     14
```

PL/Rust itself is a [`pgrx`](https://github.com/tcdi/pgrx)-based Postgres extension.  Furthermore, each `LANGUAGE
plrust` function are themselves mini-pgrx extensions. `pgrx`is a generalized framework for developing Postgres extensions with Rust.  Like this project, `pgrx`
is developed by [TCDI](https://www.tcdi.com).

The following sections discuss PL/Rusts safety guarantees, configuration settings, and installation instructions.


# Installing PL/Rust

Installing PL/Rust and especially `postgrestd` requires a normal installation of Rust via
[`rustup`](https://rustup.rs) and for the relevant locations to be writeable on the building host.
See the [Install PL/Rust](https://tcdi.github.io/plrust/install-plrust.html)
section of the documentation for notes on installing PL/Rust and its dependencies.



## Cross Compilation Support

See the
[Cross compliation](https://tcdi.github.io/plrust/install-cross-compile.html)
section of the documentation for cross-compilation details.



## Configuration

See the [PostgreSQL Configuration](https://tcdi.github.io/plrust/config-pg.html)
section of the documentation for notes on configuring PL/Rust in
`postgresql.conf`.


----


## Lints

See the [Lints section](https://tcdi.github.io/plrust/config-lints.html)
of the documentation.


## Environment Variables

See the [Environment variables section](https://tcdi.github.io/plrust/config-env-var.html)
of the documentation.



# Quickly Getting Started

To quickly get started using PL/Rust for evaluation purposes, install `cargo-pgrx` following the steps from above, then...

```bash
$ git clone https://github.com/tcdi/plrust.git
$ cd plrust/plrust
$ cargo pgrx run pg14
psql> \q

$ SCRATCH_DIR=/home/${USER}/plrust-scratch
$ cat <<-EOF >> ~/.pgrx/data-14/postgresql.conf
  plrust.work_dir = '${SCRATCH_DIR}'
EOF
$ mkdir -p scratch
$ chmod -R 777 scratch
```

Then run it for real and start writing functions!

```bash
$ cargo pgrx run pg14
psql> CREATE EXTENSION plrust;
psql> CREATE FUNCTION strlen(name TEXT) RETURNS int LANGUAGE plrust AS $$
    Ok(Some(name.unwrap().len() as i32))
$$;
psql> select strlen('Hello, PL/Rust');
strlen 
--------
     14
```


# Other Notes

In the Postgres world it seems common for procedural languages to have two styles, "trusted" and "untrusted".  The consensus is to name those as "lang" and "langu", respectively -- where the "u" is supposed to represent "untrusted" (see "plperl" v/s "plperlu" for example).

PL/Rust does not do this.  The only thing that Postgres uses to determine if a language handler is considered "trusted" is if it was created using `CREATE TRUSTED LANGUAGE`.  It does not inspect the name.

PL/Rust stores the compiled user function binaries as a `bytea` in an extension-specific table uniquely key'd with its compilation target.

As such, compiling a function with an "untrusted" version of PL/Rust, then installing the "trusted" version and trying to run that function will fail -- "trusted" and "untrusted" are considered different compilation targets and are not compatible with each other, even if the underlying hardware is exactly the same.

This does mean that it is not possible to install both "trusted" and "untrusted" versions of PL/Rust on the same Postgres database cluster.

In the future, as `postgrestd` is ported to more platforms, we will seriously consider having both `plrust` and `plrustu`.  Right now, since "trusted" is only possible on Linux `x86_64`/`aarch64`, our objective is to drive production installations to be "trusted", while allowing non-Linux developers the ability to use `LANGUAGE plrust` too.


# Security Notice

Please read the [Security](SECURITY.md) for directions on reporting a potential security issue.

# License

PL/Rust is licensed under "The PostgreSQL License", which can be found [here](LICENSE.md).
