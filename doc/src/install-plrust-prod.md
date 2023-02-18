# Install PL/Rust for Production

> This documentation is under development.

These instructions explain how to install `plrust` for production use case.
This includes creating a `plrust` binary to be installed on production
PostgreSQL instances as well as configuration and security best practices.


[https://github.com/tcdi/pgx/blob/master/CROSS_COMPILE.md](https://github.com/tcdi/pgx/blob/master/CROSS_COMPILE.md)


## Trusted install

The recommended way to install `plrust` for production database use is in
trusted mode.

Follow the steps in [Install PL/Rust for Development](./install-plrust-dev.md)
through the `cargo pgx init` step.  After running `cargo pgx init`
is when the additional steps for trusted install happen.


```bash
rustup component add llvm-tools-preview rustc-dev

cd ~/plrust/plrustc
./build.sh
mv ~/plrust/build/bin/plrustc ~/.cargo/bin/
cargo pgx install --release --features trusted -c /usr/bin/pg_config
```



## Cross compilation

Cross compilation details coming soon.

