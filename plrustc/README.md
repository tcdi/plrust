# `plrustc`

`plrustc` is the Rust compiler driver (e.g. it's a binary that uses `rustc_driver`) used by PL/Rust to implement trusted mode.

Just to give a brief overview: trusted mode combines various lints with a [custom standard library](https://github.com/tcdi/postgrestd) in an effort to meet the PostgreSQL requirements for a trusted language handler, mainly restricting all access to the underlying operating system. Note that this is only done for the user crate and not dependencies, although dependencies can be disabled in PL/Rust's configuration (and you probably want to do so if you are using "trusted" mode).

FIXME(thom): Okay I rewrote a bunch of stuff and a lot of this is somewhat less accurate. The builder is gone for now, as is, temporarially, the use of RUSTC_WRAPPER. I think we probably *do* need the builder, but.... let's see if we can replace it with something less monsterous????

## Developing

It's recommended that you use rustup for your toolchain during development. Configure things as specified in the rust-toolchain.toml at the repository root.

### IDE configuration

We have a few independent workspaces. My `rust-analyzer` configuration currently looks like this to support them.

```json
{
    "rust-analyzer.linkedProjects": ["./Cargo.toml", "plrustc/Cargo.toml"],
    "rust-analyzer.rustc.source": "discover",
    "rust-analyzer.cargo.extraEnv": { "RUSTC_BOOTSTRAP": "1" },
    "rust-analyzer.cargo.buildScripts.enable": true,
    "rust-analyzer.procMacro.enable": true,
}
```

The use of `RUSTC_BOOTSTRAP` here is unfortunate, but at the moment things are the way they are.

## Installation

Installation of PL/Rust with trusted mode is a bit hairy.

Building `plrustc` requires a specific Rust toolchain version, and the toolchain must include the `rustc-dev` component. The easiest way of achieving this is via rustup, however this generally does not do well in global installation.

1. To install globally, first determine the version of Rust that PL/Rust requires (it currently does not support more than one version for a given release, and you *must* match that version exactly). This can be determined via the [rust-toolchain.toml](https://github.com/tcdi/plrust/blob/main/rust-toolchain.toml) file.

2. Then, install Rust for that verion via an offline installer from <https://forge.rust-lang.org/infra/other-installation-methods.html#standalone-installers>.

3. Then, install the `rustc-dev` component for the same version from the distribution server. A user-friendly link to this is not available (although the steps to determine the URL are documented [here](https://forge.rust-lang.org/infra/channel-layout.html)), but this generally will be something like `https://static.rust-lang.org/dist/rustc-dev-$RUST_VERSION-$TARGET_TRIPLE.tar.gz`, where `RUST_VERSION` is the version from the other step, and TARGET_TRIPLE is either `x86_64-unknown-linux-gnu` (on x86_64 linux), and `aarch64-unknown-linux-gnu` (on aarch64 linux).

    Note: Other targets are not really supported by PL/Rust trusted mode at the moment (aside from partial checking-only support for development purposes, which lacks documentation).

4. Then, build and install `plrustc` and `postgrestd`. The build system at the root of the repository can help. `./b.sh help`. In theory, it can do a few of the other steps in this list too, but for safety I've written them down anyway.

5. After all that, you can finally install PL/Rust. I think this is done with `cargo pgx install`? Something like that. Someone should clean up this guide honestly.

## Usage

Similar to `rustc`, `plrustc` is usually not invoked directly, but instead through `cargo`. While a `plrustc`-using wrapper for cargo exists in this repository, it should not be used except for testing/development/etc.

## Details

Some additional details are provided for users who intend to run PL/Rust and plrustc under restricted environments via seccomp and/or selinux. These details are subject to change, which should be noted in documentation.

### Environment variables

Here are some `plrustc`-specific environment variables that PL/Rust or `plrustc` itself may set. This is in addition to the ones mentioned in "Sysroot Configuration" below.

The `PLRUSTC_FLAGS` variable contains flags passed by PL/Rust which are specific to `plrustc` and required for it to function. The `PLRUSTC_PASSTHROUGH` argument is also used to invoke `plrustc` as if it were plain `rustc`, which it may . FIXME(thom): `PLRUSTC_FLAGS` are not used right now.

PL/Rust is likely to invoke `plrustc` as the value of both `RUSTC` and
`RUSTC_WRAPPER`. FIXME(thom): seems like we might only need to set it as `RUSTC` now? Not sure.

### Sysroot configuration

To locate the Rust sysroot (which should have the installation of `postgrestd`), the following algorithm is used. It is very similar to the algorithm used by clippy, miri, etc. We stop at the first of these that provides a value.

1. If a `-Zplrustc-sysroot=...` argument is present in `PLRUSTC_FLAGS` then this is used. FIXME(thom): this is not true anymore.
2. If a `--sysroot` argument is provided via normal program arguments, then that value is used.
3. The runtime environment is checked.
   1. First, for `PLRUSTC_SYSROOT` and `SYSROOT` in that order of preference.
   2. Then, for rustup: If both `RUSTUP_HOME` and `RUSTUP_TOOLCHAIN` are set, then we will use the path `$RUSTUP_HOME/toolchains/$RUSTUP_TOOLCHAIN` as the sysroot.
4. If `rustc` is on the path, then `rustc --print sysroot` is invoked and that value is used.
5. The compile-time environment is checked.
   1. First, for `PLRUSTC_SYSROOT` and `SYSROOT` in that order of preference.
   2. Then, for rustup, if both `RUSTUP_HOME` and `RUSTUP_TOOLCHAIN` are set in the environment at runtime, then we will use the path `$RUSTUP_HOME/toolchains/$RUSTUP_TOOLCHAIN` as the sysroot.
6. If none of these were successful, an error is emitted and compilation will fail.

It's likely that a future version of plrustc will refine this to allow more control. In the short term this is impossible, howrever.


