# Install PL/Rust for Development

> This documentation is under development.

These instructions explain how to install `plrust` for development purposes
with PostgreSQL installed using the OS package manager.

Tested using Ubuntu 22.04.

Install all prerequisites outlined for [pgx](https://github.com/tcdi/pgx#system-requirements).
Pay special attention to PostgreSQL's build dependencies.

PostgreSQL 15 for this document is installed using `apt` and the `pgdg` repository.
See the [PostgreSQL apt wiki page](https://wiki.postgresql.org/wiki/Apt)
for instructions.

These instructions currently install `rustc`, `pgx`, and `plrust` as the
Linux `postgres` user.

Postgres needs to be given ownership of two directories for `pgx` to be able
to install.


```bash
sudo chown postgres -R /usr/share/postgresql/15/extension/
sudo chown postgres -R /usr/lib/postgresql/15/lib/
```

Switch to the Linux `postgres` user.


```bash
sudo su - postgres
```

Install `rustc` using `rustup`.

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Use `exit` to log out of the `postgres` user, and switch back to the `postgres`
user.  This ensures your terminal is using the proper `rustc` installation.


```bash
# Log in  as postgres fresh with rustc installed
exit
sudo su - postgres
```

Clone the `plrust` repo from GitHub and change into this directory.

```bash
git clone https://github.com/tcdi/plrust.git
cd plrust/plrust
```


Change into the `plrust/plrust` directory and check the version of `rustc`.


```bash
cd plrust/plrust
rustc -V
```

```
rustc 1.65.0 (897e37553 2022-11-02)
```

### A note on Rust versions

The above check of `rustc -V` is important before installing `pgx`.  
You must install `pgx` with the version of `rustc` that `plrust` expects in the
`rust-toolchain.toml`.  Failing to do so will result in a mismatched version error in
a subsequent step.

The impact of the `rust-toolchain.toml` is shown in the following code block.

```bash
~/plrust/plrust$ rustc -V
rustc 1.65.0 (897e37553 2022-11-02)
~/plrust/plrust$ cd ~/
~$ rustc -V
rustc 1.67.0 (fc594f156 2023-01-24)
```

## Install pgx

Install pgx with the `--locked` option.

```bash
cargo install cargo-pgx --locked
```

Initialize `pgx` for PostgreSQL 15 using the standard Ubuntu path to `pg_config`.

```bash
cargo pgx init --pg15 /usr/bin/pg_config
```

Install the `plrust` extension in untrusted mode.

```bash
cargo pgx install --release -c /usr/bin/pg_config
```

> Note: To use **trusted** mode add `--features "trusted"`.  You must have installed `postgrestd` and the cross-compile toolchain for this feature.

Update `postgresql.conf` -- add `plrust` to `shared_preload_libraries`

```bash
nano /etc/postgresql/15/main/postgresql.conf
```

```
shared_preload_libraries = 'plrust'
plrust.work_dir = '/tmp'
```

```bash
exit
sudo systemctl restart postgresql
```

While we're a user with `sudo`, set permissions back.

```bash
sudo chown root -R /usr/share/postgresql/15/extension/
sudo chown root -R /usr/lib/postgresql/15/lib/
```

## Try it out

Change back to `postgres` user.

```bash
sudo su - postgres
```


```bash
psql -c "CREATE DATABASE plrust;"
psql -d plrust
```

```sql
CREATE EXTENSION plrust;
```

You'll see the warning that the extension is untrusted.

```bash
WARNING:  plrust is **NOT** compiled to be a trusted procedural language
```


Create a test extension using `plrust`.



```sql
CREATE FUNCTION one()
    RETURNS INT LANGUAGE plrust
AS
$$
    Ok(Some(1))
$$;
```

## Update Development PL/Rust installation

Follow these steps to upgrade PL/Rust from GitLab to test
the latest release.  Start as a user with `sudo` access.

```bash
sudo chown postgres -R /usr/share/postgresql/15/extension/
sudo chown postgres -R /usr/lib/postgresql/15/lib/

sudo su - postgres
cd plrust
git pull
cd plrust
cargo pgx install --release -c /usr/bin/pg_config

exit

# Restart Postgres, plrust is in shared_preload_libraries
sudo systemctl restart postgresql

sudo chown root -R /usr/share/postgresql/15/extension/
sudo chown root -R /usr/lib/postgresql/15/lib/
```


