# PostgreSQL Configuration for PL/Rust

PL/Rust has two **required** configuration options and a variety of non-required options.
These options are set in the standard `postgresql.conf` configuration file used
by PostgreSQL.

PL/Rust has two required configuration options in untrusted and trusted installations.
Using PL/Rust with cross compilation support has a 3rd required configuration option.
Failure to set these variables
will cause `plrust` extension to not function.


## Required


PL/Rust has two PostgreSQL configuration options that are always required for use,`shared_preload_libraries` and `plrust.work_dir`.


#### `shared_preload_libraries` (string)

The [`shared_preload_libraries` entry](https://www.postgresql.org/docs/current/runtime-config-client.html)
needs to include `plrust`. This is a comma separated list of libraries that
need to be pre-loaded in order to operate properly.

```bash
shared_preload_libraries = 'plrust'
```



#### `plrust.work_dir` (string)

The `plrust.work_dir` must be set to location for PL/Rust to save
necessary intermediate files. This path must be writable by the user running
the PostgreSQL process, typically `postgres` on common Linux distributions.

```bash
plrust.work_dir = '/tmp'
```



## Additional Configuration Options



#### `plrust.allowed_dependencies` (string)

Define the path to a `toml` file with an allow-list of Rust crates and versions when creating
PL/Rust functions.
When `plrust.allowed_dependencies` is not defined, all Rust crates are allowed
when creating PL/Rust functions.

Consider a file `/path/to/plrust_allowed.toml` with the following contents.

```toml
foo = "1.1.5"
```

The configuration to restrict crates looks like the following example.

```bash
plrust.allowed_dependencies = /path/to/plrust_allowed.toml
```


#### `plrust.path_override` (string)

Set this if `cargo` and `cc` are not in the postmaster's `$PATH`.

```bash
plrust.path_override = '/special/path/to/.cargo/bin:/usr/bin'
```


#### `plrust.trusted_pgrx_version` (string)

The version of the `plrust-trusted-pgrx` crate from crates.io to use when
compiling user functions. This typically should not need to be manually set.


```bash
plrust.trusted_pgrx_version = '1.1.3'
```


#### `plrust.tracing_level` (string)

A [tracing directive](https://docs.rs/tracing-subscriber/0.3.11/tracing_subscriber/filter/struct.EnvFilter.html).

```bash
plrust.tracing_level = 'info'
```



## Required for Cross Compilation

#### `plrust.compilation_targets` (string)

Using PL/Rust with cross compilation requires the `plrust.compilation_targets`
configuration option.  This is required for PL/Rust to cross compile user functions.
The `plrust.compilation_targets` option is a comma-separated list of values,
of which only `x86_64` and `aarch64` are currently supported.


```bash
plrust.compilation_targets = 'x86_64, aarch64'
```

For PL/Rust to cross compile user functions it needs to know which CPU architectures via
`plrust.compilation_targets`. This is a comma-separated list of values, of which only `x86_64` and `aarch64` are
currently supported.

#### `plrust.{arch}_linker` (string)

This is the name of the linker `rustc` should use on for cross-compile.
The architecture linker names have sensible defaults and shouldn't need to be be
changed (unless the host is some esoteric Linux distribution we have not encountered yet).

```bash
plrust.x86_64_linker = 'x86_64_linux_gnu_gcc'
plrust.aarch64_linker = 'aarch64_linux_gnu_gcc'
```



#### `plrust.{arch}_pgrx_bindings_path` (string)

The `plrust.{arch}_pgrx_bindings_path` settings are actually required but PL/Rust will happily cross compile without them. If unspecified,
PL/Rust will use the pgrx bindings of the host architecture for the cross compilation target architecture too. In other words, if the host 
is `x86_64` and PL/Rust is configured to cross compile to `aarch64` and the `plrust.aarch64_pgrx_bindings_path` is *not* configured, it'll
blindly use the bindings it already has for `x86_64`.  This may or may not actually work.

To get the bindings, install `cargo-pgrx` on the other system and run `cargo pgrx cross pgrx-target`. That'll generate a tarball. Copy that back 
to the primary host machine and `untar` it somewhere (PL/Rust doesn't care where), and use that path as the configuration setting.

Note that it is perfectly fine (and really, expected) to set all of these configuration settings on both architectures.
PL/Rust will silently ignore the one for the current host.  In other words, plrust only uses them when cross compiling for 
the other architecture.


## Lints

There are two The PL/Rust configuration options related to lints. **These options
should not be changed.**
Altering these configuration options has two main negative side effects.
Disabling any of the pre-configured lints **removes any and all expectation**
of PL/Rust being trusted.
Changing this option can also prevent upgrading PL/Rust.

See the [Lints Configuration](config-lints.md) section for more details about the
purpose of the Lints.


#### `plrust.compile_lints` (string)

A comma-separated list of Rust lints to apply to every user function.

```bash
plrust.compile_lints = 'plrust_extern_blocks, plrust_lifetime_parameterized_traits, implied_bounds_entailment, unsafe_code, plrust_filesystem_macros, plrust_env_macros, plrust_external_mod, plrust_fn_pointers, plrust_async, plrust_leaky, plrust_print_macros, plrust_stdio, unknown_lints, deprecated, suspicious_auto_trait_impls, unaligned_references, soft_unstable, plrust_autotrait_impls'
```


#### `plrust.required_lints` (string)

A comma-separated list of Rust lints that are required to have been applied to a user function before PL/Rust will load the library and execute the function.

The value of `plrust.required_lints` defaults to `plrust.compile_lints`.

