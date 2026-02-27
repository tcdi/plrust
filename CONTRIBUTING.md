## Dev Environment Setup

System Requirements:
- Rustup (or equivalent toolchain manager like Nix). Users may be able to use distro toolchains, but you won't get far without a proper Rust toolchain manager.
- Otherwise, same as the user requirements.


If you want to be ready to open a PR, you will want to run
```bash
git clone --branch develop "https://github.com/tcdi/plrust"
cd plrust
```
That will put you in a cloned repository with the **`develop`** branch opened, which is the one you will be opening pull 
requests against in most cases.

After cloning the repository the first things you need to do are install `plrustc` and, if on Linux, perhaps `postgrestd`:

```bash

## install plrustc
cd plrustc
./build.sh
cp ../build/bin/plrustc ~/.cargo/bin

## install postgrestd
cd ../plrust/
./build
```

PL/Rust is a [pgrx](https://github.com/tcdi/pgrx) extension and will need `cargo-pgrx` installed:

```bash
cargo install cargo-pgrx --locked
```

## Pull Requests (PRs)

- Pull requests for new code or bugfixes should be submitted against the **`develop`** branch
- All pull requests against `develop` will be squashed on merge
- Tests are *expected* to pass before merging
- Diffs in `Cargo.lock` should be checked in

### Adding Dependencies

If a new crate dependency is required for a pull request, and it can't or should not be marked optional and behind some 
kind of feature flag, then it should have its reason for being used stated in the Cargo.toml it is added to. This can 
be "as a member of a category", in the case of e.g. error handling:

```toml
# error handling and logging
eyre = "0.6.8"
thiserror = "1.0"
tracing = "0.1.34"
tracing-error = "0.2.0"
```

It can be as notes for the individual dependencies:
```toml
once_cell = "1.10.0" # polyfill until std::lazy::OnceCell stabilizes
```

Or it can be both:

```toml
# exposed in public API
atomic-traits = "0.3.0" # PgAtomic and shmem init
bitflags = "1.3.2" # BackgroundWorker
bitvec = "1.0" # processing array nullbitmaps
```

You do not need exceptional justification notes in your PR to justify a new dependency as your code will, in most cases, 
self-evidently justify the use of the dependency. PL/Rust uses the normal Rust approach of using dependencies based on their
ability to improve correctness and make features possible. It does not reimplement things already available in the Rust 
ecosystem unless the addition is trivial (do not add custom derives to save 5~10 lines of code in one site) or the ecosystem 
crates are not compatible with Postgres (unfortunately common for Postgres data types).

## Releases

On a new PL/Rust release, **`develop`** will be merged to **`main`** via merge commit.
<!-- it's somewhat ambiguous whether we do this for stable or also "release candidate" releases -->

### Release Candidates AKA Betas
PL/Rust prefers using `x.y.z-{alpha,beta}.n` format for naming release candidates,
starting at `alpha.0` if the new release candidate does not seem "feature complete",
or at `beta.0` if it is not expected to need new feature work. Remember that `beta` will supersede `alpha` in versions 
for users who don't pin a version.

## Licensing

You agree that all code you submit in pull requests to https://github.com/tcdi/plrust/pulls
is offered according to the PostgreSQL License, thus may be freely licensed and sublicensed,
and that you are satisfied with the existing copyright notice as of opening your PR, which is:

```
Portions Copyright 2020-2021 ZomboDB, LLC.

Portions Copyright 2021-2025 Technology Concepts & Design, Inc.
```

It is the latter to which copyrights for all merged code is assigned.
