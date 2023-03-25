# PL/Rust Cross Compilation

This section explains difference required during PL/Rust installation to support
PL/Rust cross compilation.

## Install cross compile dependencies

As a Linux user with `sudo` access, install additional prerequisites.

```bash
sudo apt install crossbuild-essential-arm64 crossbuild-essential-amd64
```

Change to the `postgres` user for the next steps.

```
sudo su - postgres
```

Install the Rust targets for `aarch64` and `x86_64`.
These are necessary to cross compile `postgrestd` and PL/Rust user functions.


```bash
cd plrust/plrust
rustup component add llvm-tools-preview rustc-dev
rustup target install aarch64-unknown-linux-gnu
rustup target install x86_64-unknown-linux-gnu
```

When the above completes, run the `postgrestd` build script.
This example assumes that the `pg_config` binary from Postgresql 15 is on your `$PATH`.
If v15 is not your intended Postgres version, change it to the proper major version number.
See the [Install PL/Rust](install-plrust.md) section for examples of this.


```bash
PG_VER=15 \
    STD_TARGETS="x86_64-postgres-linux-gnu aarch64-postgres-linux-gnu" \
    ./build
```

> The above environment variables are the default... you can just run `./build`.


This will take a bit of time as it clones the `postgrestd` repository,
builds it for two architectures, and finally runs PL/Rust's entire test suite
in "trusted" mode.


## Configuration

The `plrust.compilation_targets` must be set in `postgresql.conf` in order for
PL/Rust to cross compile user functions.
This is a comma-separated list of values, of which only `x86_64` and `aarch64` are
currently supported.  See the [PostgreSQL Configuration](config-pg.md) section
for more about configuring PL/Rust.


The architecture linker names have sane defaults and shouldn't need to be be changed (unless the host is some
esoteric Linux distro we haven't encountered yet).

The `plrust.{arch}_pgx_bindings_path` settings are actually required but PL/Rust will happily cross compile without them. If unspecified,
PL/Rust will use the pgx bindings of the host architecture for the cross compilation target architecture too. In other words, if the host 
is `x86_64` and PL/Rust is configured to cross compile to `aarch64` and the `plrust.aarch64_pgx_bindings_path` is *not* configured, it'll
blindly use the bindings it already has for `x86_64`.  This may or may not actually work.

To get the bindings, install `cargo-pgx` on the other system and run `cargo pgx cross pgx-target`. That'll generate a tarball. Copy that back 
to the primary host machine and untar it somewhere (PL/Rust doesn't care where), and use that path as the configuration setting.

Note that it is perfectly fine (and really, expected) to set all of these configuration settings on both architectures.
plrust will silently ignore the one for the current host.  In other words, plrust only uses them when cross compiling for 
the other architecture.

