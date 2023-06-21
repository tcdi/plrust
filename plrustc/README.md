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

