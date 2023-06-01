[![crates.io badge](https://img.shields.io/crates/v/plrust-trusted-pgrx.svg)](https://crates.io/crates/plrust-trusted-pgrx)
[![docs.rs badge](https://docs.rs/pgrx/badge.svg)](https://docs.rs/plrust-trusted-pgrx)

`plrust-trusted-pgrx` is a re-export crate based on [`pgrx`](https://crates.io/crates/pgrx) which exports the minimal set
of capabilities necessary to compile [`plrust`](https://github.com/tcdi/plrust) user functions along with safe access to
various parts of Postgres including some data types, logging, Spi, and triggers.

You might be tempted to use this for your own pgrx extension development, but you shouldn't.  It's intended for use only
with plrust.