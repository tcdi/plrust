# Update PL/Rust

This section explains how to update PL/Rust installations.  This assumes
you installed PL/Rust following our [installation guide](./install-plrust.md).

## Update process


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

## Rust versions

See the section(s) about Rust versions
the the [Install PL/Rust](./install-plrust.md) section.
Pay special attention to the versions defined by PL/Rust, and your
system defaults for `rustc` and `rustup`.


