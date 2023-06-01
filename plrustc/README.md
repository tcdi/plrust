# `plrustc`

`plrustc` is the Rust compiler driver (e.g. it's a binary that uses `rustc_driver`) used by PL/Rust to implement trusted mode.

Just to give a brief overview: trusted mode combines various lints with a [custom standard library](https://github.com/tcdi/postgrestd) in an effort to meet the PostgreSQL requirements for a trusted language handler, mainly restricting all access to the underlying operating system. Note that this is only done for the user crate and not dependencies, although dependencies can be disabled in PL/Rust's configuration (and you probably want to do so if you are using "trusted" mode).

## Developing

It's recommended that you use rustup for your toolchain during development. Configure things as specified in the rust-toolchain.toml at the repository root.

`plrustc` is not part of the overall `plrust` workspace for at least two reasons:

- `-Clink-args=-Wl,-undefined,dynamic_lookup` causes problems for it
- because it needs `rustc-dev` and `llvm-tools-preview` installed

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

## Usage

Similar to `rustc`, `plrustc` is usually not invoked directly, but instead through `cargo`.

## Details

Some additional details are provided for users who intend to run PL/Rust and plrustc under restricted environments via seccomp and/or SELinux. These details are subject to change, although if that occurs it will be noted in the changelog.

### Sysroot configuration

To locate the Rust sysroot (which should have the installation of `postgrestd`), the following algorithm is used. It is very similar to the algorithm used by clippy, miri, etc. We stop at the first of these that provides a value.

1. If a `--sysroot` argument is provided via normal program arguments, then that value is used.
2. The runtime environment is checked.
   1. First, for `PLRUSTC_SYSROOT` and `SYSROOT` in that order of preference.
   2. Then, for rustup: If both `RUSTUP_HOME` and `RUSTUP_TOOLCHAIN` are set, then we will use the path `$RUSTUP_HOME/toolchains/$RUSTUP_TOOLCHAIN` as the sysroot.
3. If `rustc` is on the path, then `rustc --print sysroot` is invoked and that value is used.
4. The compile-time environment is checked.
   1. First, for `PLRUSTC_SYSROOT` and `SYSROOT` in that order of preference.
   2. Then, for rustup, if both `RUSTUP_HOME` and `RUSTUP_TOOLCHAIN` are set in the environment at runtime, then we will use the path `$RUSTUP_HOME/toolchains/$RUSTUP_TOOLCHAIN` as the sysroot.
5. If none of these were successful, an error is emitted and compilation will fail.

It's likely that a future version of plrustc will refine this to allow more control. In the short term this is impossible, howsever.
