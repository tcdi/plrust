[![crates.io badge](https://img.shields.io/crates/v/plrust-trusted-pgx.svg)](https://crates.io/crates/plrust-trusted-pgx)
[![docs.rs badge](https://docs.rs/pgx/badge.svg)](https://docs.rs/plrust-trusted-pgx)

`plrust-trusted-pgx` is a re-export crate based on [`pgx`](https://crates.io/crates/pgx) which exports the minimal set
of capabilities necessary to compile [`plrust`](https://github.com/tcdi/plrust) user functions along with safe access to
various parts of Postgres including some data types, logging, Spi, and triggers.

You might be tempted to use this for your own pgx extension development, but you shouldn't.  It's intended for use only
with plrust.