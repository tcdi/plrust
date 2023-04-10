# Install PL/Rust Prerequisites

These instructions explain how to install PL/Rust on a typical OS PostgreSQL
installation installed using the OS' package manager. These instructions
include steps for [trusted and untrusted](trusted-untrusted.md)
`plrust` and are tested using Ubuntu 22.04 and PostgreSQL 15.
PostgreSQL 15 for this document is installed using `apt` using
the `pgdg` repository.
See the [PostgreSQL apt wiki page](https://wiki.postgresql.org/wiki/Apt)
for instructions.

Steps to install PL/Rust:

* Prerequisites
* Install Rust
* Install pgx
* Install PL/Rust
* Create amazing things!

## Prerequisites

PL/Rust requires PostgreSQL and all prerequisites outlined for
[pgx](https://github.com/tcdi/pgx#system-requirements)
are installed.  

PL/Rust also requires that any databases in which it's created is `UTF8`.  Postgres' default encoding is determined
by the locale of the environment when `initdb` is first run.  Depending on your operating system configuration, this may 
not resolve to `UTF8`.

[Building PL/Rust from source](https://wiki.postgresql.org/wiki/Compile_and_Install_from_source_code) requires 
installing `cargo-pgx` which requires a development toolchain capable of building Postgres itself.


### Permissions

Installing PL/Rust with these instructions installs `rustc`, `pgx`,
and `plrust` as the Linux `postgres` user.  The `postgres` user
is created during the standard PostgreSQL installation via `apt`.
For `pgx` to successfully install `plrust`, the `postgres`
user needs ownership of the `extension` and `lib` directories.
The standard Ubuntu locations are indicated below.


```bash
sudo chown postgres -R /usr/share/postgresql/15/extension/
sudo chown postgres -R /usr/lib/postgresql/15/lib/
```

These permissions are later reset back to being owned by `root`
in the [Reset Permissions](install-plrust.md#reset-permissions) section.

## Install `rustc`

Installing PL/Rust requires that the `rustc` compiler is available
to the user installing it.
Switch to the `postgres` Linux user and change into its home directory.


```bash
sudo su - postgres
```

The typically installation for `rustc` uses `curl` and `rustup`.
If you want to install `rustc` without using `rustup` see the
[Other Rust installation methods](https://forge.rust-lang.org/infra/other-installation-methods.html)
page.


```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

The `rustup` installer prompts for an installation choice.  The
default installation (1) should work for most use cases.

```bash
1) Proceed with installation (default)
2) Customize installation
3) Cancel installation
```


After installing rust, use `exit` to log out and back in to the `postgres`
account.  This ensures your terminal is using the newly installed
`rustc` installation.

```bash
# Log in  as postgres fresh with rustc installed
exit
sudo su - postgres
```

### Clone `plrust` and check Rust version

PL/Rust is installed from source code using pgx.  This installation
requires that pgx is compiled using a specific version of `rustc`.
The `rustc` version required for PL/Rust is defined in the project's
[`rust-toolchain.toml`](https://github.com/tcdi/plrust/blob/main/rust-toolchain.toml).
The steps below ensure the proper versions are used.

Clone the `plrust` repo from GitHub and change into the `plrust/plrust`
directory. Running `rustc -V` in this location is used to verify
the version reported is by `rustc -V` is the version defined by PL/Rust.

```bash
git clone https://github.com/tcdi/plrust.git
cd plrust/plrust
rustc -V
```

The output from `rustc -V` should look similar to the following example.

```
rustc 1.67.1 (d5a82bbd2 2023-02-07)
```

Use `rustup default` to check that the explicit version of `rustc` is
selected.
You need to see the version number reported in by `rustc -V` in
your `rustup default` output.


```bash
rustup default
```

The expected output is below.

```
1.67.1-x86_64-unknown-linux-gnu (default)
```

If `rustup default` returns a different version number or `stable`,
set the default version as shown below and check that the output
updates accordingly.


```bash
rustup default 1.67.1
rustup default
```



### Be careful with Rust versions


> **WARNING!** The `stable` version of `rustc` cannot be used to install Trusted PL/Rust.  This is the case even when the `stable` version is identical to the tagged version number, such as `1.67.1`.


The above checks of `rustc -V` and `rustup default` are important to
follow before installing pgx and PL/Rust.
You must install `pgx` with the version of `rustc` that `plrust` expects
in the `rust-toolchain.toml`.  Failing to do so will result in a
mismatched version error in a subsequent step.

A misconfigured `rustup default` results in
errors when creating functions with trusted PL/Rust. The error can
manifest as a problem in the `postgrestd` linking with the following error.
This happens because Rust makes a distinction between the latest stable
version of Rust, and the actual version of the stable release (e.g. 1.67.1),
even when they refer to the same release.

```bash
Error loading target specification: Could not find specification for target "x86_64-postgres-linux-gnu".
```


## Install pgx

The PL/Rust extension is built and installed
[using pgx](https://github.com/tcdi/pgx).
Install pgx with the `--locked` option. This step takes a few
minutes.

```bash
cargo install cargo-pgx --locked
```

Pgx needs to be initialized for use with the PostgreSQL installation.
This is done using `pgx init`.  This step needs to know where your
`pg_config` file is located at.  If you have a standard Ubuntu
`apt` installation of PostgreSQL with a single version of PostgreSQL
installed you can use the generic
`/usr/bin/pg_config` path.  

```bash
cargo pgx init --pg15 /usr/bin/pg_config
```

Output from `cargo pgx init` looks like the following example.
You may notice it mentions information about a new data directory under your
user's `~/.pgx/` directory. This **does not replace** your PostgreSQL instance's
data directory. The `~/.pgx/data-15/` directory is there in case you run
`cargo pgx run pg15`, which would use this custom data directory, not your installation's data directory.

```
   Validating /usr/bin/pg_config
 Initializing data directory at /var/lib/postgresql/.pgx/data-15
```



The generic `pg_config` used above will not work
for all installations, such as if you have both PostgreSQL 14 and 15
installed on one instance.
In these cases you should specify the exact `pg_config`
file for your installation.

```bash
cargo pgx init --pg14 /usr/lib/postgresql/14/bin/pg_config
```

The instructions on this page have setup the prerequisite software required to
install PL/Rust.  The next section, [Install PL/Rust](install-plrust.md),
finishes the installation process.

