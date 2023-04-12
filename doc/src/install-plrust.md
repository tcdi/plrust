# Install PL/Rust

This section provides steps on how to proceed with installing PL/Rust.  There
are three ways to install PL/Rust.
Most users will want to install [trusted PL/Rust](#trusted-install).

* [Untrusted](#untrusted-install)
* [Trusted](#trusted-install)
* [Trusted plus cross compilation](install-plrust.md#trusted-installation-plus-cross-compilation)


These instructions assume you have followed the [Install Prerequisites](install-prerequisites.md)
section and are logged in as the `postgres` Linux user.
Install PL/Rust by following the installation steps under your method of choice
below.  Then visit
[the configuration subsection](install-plrust.html#configure-and-restart-postgresql)
and give [PL/Rust a try](install-plrust.html#try-it-out)!


### Untrusted install

To install **untrusted** PL/Rust use `cargo pgx install`
without `--features trusted`.  See the [trusted install](#trusted-install) if you
wish to install the trusted PL/Rust instead.

```bash
cargo pgx install --release -c /usr/bin/pg_config
```

Continue on to [configuring PostgreSQL](install-plrust.html#configure-and-restart-postgresql)
for PL/Rust.


### Trusted install

The trusted installation requires `postgrestd` and a few additional
Rust dependencies.  First install the additional dependencies.  This example
uses `x86_64` and ensures the target is installed.  If you are using `aarch64`,
update the command accordingly.


```bash
rustup component add llvm-tools-preview rustc-dev
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
install `postgrestd`.  This example is for installing PL/Rust on `x86_64`
architecture, switch to `aarch64` if using that architecture instead.

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

Continue on to [configuring PostgreSQL](install-plrust.html#configure-and-restart-postgresql)
for PL/Rust.

### Choosing a different `plrust-trusted-pgx` dependency at compile time

When a user creates a `LANGUAGE plrust` function, PL/Rust first generates a small Cargo crate for the function.  That
crate has a dependency on `plrust-trusted-pgx`.  By default, `plrust-trusted-pgx` comes from crates.io, using the same 
version as PL/Rust itself.

It is possible to override this dependency when compiling PL/Rust itself so that PL/Rust will use a different 
`plrust-trusted-pgx` crate.  To do this, set an environment variable named `PLRUST_TRUSTED_PGX_OVERRIDE` to the
full "Cargo.toml"-compatible dependency line, like so:

```shell
PLRUST_TRUSTED_PGX_OVERRIDE="pgx = { path = '~/code/plrust/plrust-trusted-pgx', package='plrust-trusted-pgx' }" \
cargo pgx install --release --features trusted -c /usr/bin/pg_config
```

This will instead compile all user functions using this specific `plrust-trusted-pgx`, not the default on crates.io.
Generally, changing the `plrust-trusted-pgx` dependency is only useful for PL/Rust development and CI, not for production 
deployments, but is worth mentioning as the environment variable *will* influence how user functions are compiled.

It may also be useful for providing a local patch to `plrust-trusted-pgx` if such a need were to arise.

### Trusted installation plus cross compilation

Adding cross compilation support to PL/Rust requires a few minor changes to the
[trusted installation](#trusted-install) steps above.  This section only highlights
the changes to make for cross compile support, not the full process.


As a Linux user with `sudo` access, install these additional prerequisites.


```bash
sudo apt install crossbuild-essential-arm64 crossbuild-essential-amd64
```

The normal trusted install uses `rustup` to install one architecture target.
Cross compilation support requires both.

```bash
rustup component add llvm-tools-preview rustc-dev
rustup target install aarch64-unknown-linux-gnu
rustup target install x86_64-unknown-linux-gnu
```


Update the `STD_TARGETS` used when building `postgrestd` to include both architectures.
This step will take longer with cross compilation then only one architectures, as
it is required to double some of the work.

```bash
PG_VER=15 \
    STD_TARGETS="x86_64-postgres-linux-gnu aarch64-postgres-linux-gnu" \
    ./build
```

> The above environment variables are the default... you can just run `./build`.  `PG_VER=15` currently represents the latest released PostgreSQL version. 




## Configure and restart PostgreSQL

The PostgreSQL configuration in `postgresql.conf` must be updated for PL/Rust
to function. This section illustrates the minimum required changes so PL/Rust
will function. 
See the [PostgreSQL configuration](./config-pg.md) section for more configuration details.

PL/Rust requires `shared_preload_libraries` includes `plrust` and that you
define `plrust.work_dir`.

> NOTE:  PL/Rust with cross compilation support also requires `plrust.compilation_targets`.

Edit the PostgreSQL configuration file still as the `postgres` Linux user.

```bash
nano /etc/postgresql/15/main/postgresql.conf
```

Update the configuration with these items.  Note that `shared_preload_libraries`
might already be set with a value before you add `plrust`.  Use a comma separated
list of extensions to include multiple libraries in this configuration option.

```
shared_preload_libraries = 'plrust'
plrust.work_dir = '/tmp'
```

The PostgreSQL service needs to be restarted for the configuration changes
to take effect. Exit the `postgres` user and restart the PostgreSQL service.
 
```bash
exit
sudo systemctl restart postgresql
```

## Reset permissions

In order to install the PL/Rust extension as the `postgres` users permissions
were updated in the [Permissions section](install-prerequisites.html#permissions)
of the Install PL/Rust Prerequisites section.
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
CREATE OR REPLACE FUNCTION plrust.one()
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



