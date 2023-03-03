# Update PL/Rust

This section explains how to update PL/Rust installations.  This assumes
you installed PL/Rust following our [installation guide](./install-plrust.md) and pgx and PL/Rust are installed using the `postgres` Linux user.

## Update pgx

A PL/Rust update is often accompanied by an update of the underlying
`pgx` project.  Install the latest version of pgx.
Changing into the plrust folder ensures the `rustc` version used
for installation is the same required by PL/Rust.

Start as a user with `sudo` access.


```bash
sudo chown postgres -R /usr/share/postgresql/15/extension/
sudo chown postgres -R /usr/lib/postgresql/15/lib/
```



```bash
sudo su - postgres
cd ~/plrust
git pull
cargo install cargo-pgx --version 0.7.2 --locked

cd ~/plrust/plrust

exit
```



## Update PL/Rust


Follow these steps to upgrade PL/Rust from GitLab to test
the latest release.  

```bash
cargo pgx install --release -c /usr/bin/pg_config
```


Exit out of `postgres` user back to user with sudo.

```bash
exit
```

Restart Postgres, required b/c plrust is in `shared_preload_libraries`.
Set permissions back to default.

```bash
sudo systemctl restart postgresql

sudo chown root -R /usr/share/postgresql/15/extension/
sudo chown root -R /usr/lib/postgresql/15/lib/
```

## Rust versions

See the section(s) about Rust versions
the the [Install PL/Rust](./install-plrust.md) section.
Pay special attention to the versions defined by PL/Rust, and your
system defaults for `rustc` and `rustup`.


