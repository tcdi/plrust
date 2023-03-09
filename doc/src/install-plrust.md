# Install PL/Rust

> This documentation is under development.

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
Pay special attention to
[PostgreSQL's build dependencies](https://wiki.postgresql.org/wiki/Compile_and_Install_from_source_code).


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
in the [Reset Permissions](#reset-permissions) section.

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

The generic `pg_config` used above will not work
for all installations, such as if you have both PostgreSQL 14 and 15
installed on one instance.
In these cases you should specify the exact `pg_config`
file for your installation.

```bash
cargo pgx init --pg14 /usr/lib/postgresql/14/bin/pg_config
```


## Install PL/Rust

This section provides steps on how to proceed with both trusted and
untrusted installations of PL/Rust.
Most users will want to install trusted PL/Rust.

> Only follow the instructions under "Trusted install" or "Untrusted install."  Do not run both.


### Trusted install

The trusted installation requires `postgrestd` and a few additional
dependencies.  First install the additional dependencies.

> To install untrusted PL/Rust skip this sub-section and go to the Untrusted install section.

```bash
rustup component add llvm-tools-preview rustc-dev
rustup target install aarch64-unknown-linux-gnu
rustup target install x86_64-unknown-linux-gnu
```

Change into the `plrust/plrustc` directory to build `plrustc`.
Move the generated binary into `~/.cargo/bin/`.

```bash
cd ~/plrust/plrustc
./build.sh
mv ~/plrust/build/bin/plrustc ~/.cargo/bin/
```

> Note:  The path `~/.cargo/bin/` is the default path used by PL/Rust. This can be overridden using `plrust.PATH_override`, see [PostgreSQL Config](./config-pg.md).


Change into the `plrust/plrust/` directory and run the build process to
install `postgrestd`.

```bash
cd ~/plrust/plrust
PG_VER=15 \
    STD_TARGETS="x86_64-postgres-linux-gnu " \
    ./build
```

The above step can take quite a few minutes to
install `postgrestd` and run the associated tests.
It is not uncommon to see output like the following during the
test process.

```bash
test tests::tests::pg_plrust_aggregate has been running for over 60 seconds
```


The final step for trusted PL/Rust installation is to use
`cargo pgx install` with `--features trusted`.

```bash
cargo pgx install --release --features trusted -c /usr/bin/pg_config
```

### Untrusted install

To install untrusted PL/Rust use `cargo pgx install`.

```bash
cargo pgx install --release -c /usr/bin/pg_config
```


## Configure and restart PostgreSQL

The PostgreSQL configuration in `postgresql.conf` must be updated for PL/Rust
to function. Add `plrust` to `shared_preload_libraries`
and define `plrust.work_dir`.  See the [PostgreSQL configuration](./config-pg.md) section for more configuration details.

```bash
nano /etc/postgresql/15/main/postgresql.conf
```

Configuration items to update.

```
shared_preload_libraries = 'plrust'
plrust.work_dir = '/tmp'
```

The PostgreSQL service needs to be restarted for the configuration changes
to take effect. Exit the `postgres` user and restart the service.
 
```bash
exit
sudo systemctl restart postgresql
```

## Reset permissions

Change the permissions for the `extension` and `lib` folders back
to being owned by the `root` user.

```bash
sudo chown root -R /usr/share/postgresql/15/extension/
sudo chown root -R /usr/lib/postgresql/15/lib/
```

## Try it out

Create a `plrust` database and connect to the `plrust` database
using `psql`.


```bash
sudo -u postgres psql -c "CREATE DATABASE plrust;"
sudo -u postgres psql -d plrust
```

Create the `plrust` extension.


```sql
CREATE EXTENSION plrust;
```


If you installed the untrusted PL/Rust you will be warned of that detail
in this step.

```bash
WARNING:  plrust is **NOT** compiled to be a trusted procedural language
```

The following example creates a `plrust` function named `plrust.one()`
that simply returns the integer 1.


```sql
CREATE FUNCTION plrust.one()
    RETURNS INT LANGUAGE plrust
AS
$$
    Ok(Some(1))
$$;
```

Using a function created with PL/Rust is the same as using any other
PostgreSQL function.  A scalar function like `plrust.one()` can
be used simply like below.


```sql
SELECT plrust.one();
```

```
┌─────┐
│ one │
╞═════╡
│   1 │
└─────┘
```



