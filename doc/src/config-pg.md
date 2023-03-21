# PostgreSQL configuration

PL/Rust has a two **required** configuration options and a variety of non-required options.
PL/Rust **must** be configured as a `shared_preload_libraries` entry in `postgresql.conf`.
PL/Rust also requires `plrust.work_dir` to save intermediate files.

```
shared_preload_libraries = 'plrust'
plrust.work_dir = '/tmp'
```


Failure to set these required variables will cause `plrust` extension to not function.

The PL/Rust-specific configuration options are in the following table.


| Option                             | Type   | Description                                                                                                                                                   | Required | Default                                                                                                              |
|------------------------------------|--------|---------------------------------------------------------------------------------------------------------------------------------------------------------------|----------|----------------------------------------------------------------------------------------------------------------------|
| `plrust.work_dir`                  | string | The directory where pl/rust will build functions with cargo.                                                                                                  | yes      | <none>                                                                                                               |
| `plrust.PATH_override`             | string | If `cargo` and `cc` aren't in the `postmaster`'s `$PATH`, set this.                                                                                           | no       | environment or `~/.cargo/bin:/usr/bin` if `$PATH` is unset                                                           |
| `plrust.tracing_level`             | string | A [tracing directive][docs-rs-tracing-directive].                                                                                                             | no       | `'info'`                                                                                                             |
| `plrust.compilation_targets`       | string | Comma separated list of CPU targets (x86_64, aarch64).                                                                                                        | no       | <none>                                                                                                               |
| `plrust.x86_64_linker`             | string | Name of the linker `rustc` should use on fo cross-compile.                                                                                                    | no       | `'x86_64_linux_gnu_gcc'`                                                                                             |
| `plrust.aarch64_linker`            | string | Name of the linker `rustc` should use on for cross-compile.                                                                                                   | no       | `'aarch64_linux_gnu_gcc'`                                                                                            |
| `plrust.x86_64_pgx_bindings_path`  | string | Path to output from `cargo pgx cross pgx-target` on x86_64.                                                                                                   | no-ish   | <none>                                                                                                               |
| `plrust.aarch64_pgx_bindings_path` | string | Path to output form `cargo pgx cross pgx-target` on aarch64.                                                                                                  | no-ish   | <none>                                                                                                               |
| `plrust.compile_lints`             | string | A comma-separated list of Rust lints to apply to every user function.                                                                                         | no       | `'plrust_extern_blocks, plrust_lifetime_parameterized_traits, implied_bounds_entailment, unsafe_code, plrust_filesystem_macros, plrust_env_macros, plrust_external_mod, plrust_fn_pointers, plrust_async, plrust_leaky, plrust_print_macros, plrust_stdio, unknown_lints, deprecated, suspicious_auto_trait_impls, unaligned_references, soft_unstable, plrust_autotrait_impls'` |
| `plrust.required_lints`            | string | A comma-separated list of Rust lints that are required to have been applied to a user function before PL/Rust will load the library and execute the function. | no       | defaults to whatever `plrust.compile_lints` happens to be                                                            |            
| `plrust.trusted_pgx_version`       | string | The version of the [`plrust-trusted-pgx`](https://crates.io/crates/plrust-trusted-pgx) crate from crates.io to use when compiling user functions              |

For PL/Rust to cross compile user functions it needs to know which CPU architectures via
`plrust.compilation_targets`. This is a comma-separated list of values, of which only `x86_64` and `aarch64` are
currently supported.

The architecture linker names have sane defaults and shouldn't need to be be changed (unless the host is some
esoteric Linux distro we haven't encountered yet).

The `plrust.{arch}_pgx_bindings_path` settings are actually required but PL/Rust will happily cross compile without them. If unspecified,
PL/Rust will use the pgx bindings of the host architecture for the cross compilation target architecture too. In other words, if the host 
is `x86_64` and PL/Rust is configured to cross compile to `aarch64` and the `plrust.aarch64_pgx_bindings_path` is *not* configured, it'll
blindly use the bindings it already has for `x86_64`.  This may or may not actually work.

To get the bindings, install `cargo-pgx` on the other system and run `cargo pgx cross pgx-target`. That'll generate a tarball. Copy that back 
to the primary host machine and untar it somewhere (plrust doesn't care where), and use that path as the configuration setting.

Note that it is perfectly fine (and really, expected) to set all of these configuration settings on both architectures.
plrust will silently ignore the one for the current host.  In other words, plrust only uses them when cross compiling for 
the other architecture.