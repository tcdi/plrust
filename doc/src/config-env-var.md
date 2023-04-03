# Environment variables


As part of PL/Rust's function compilation machinery, and in conjunction with `pgx` which does the hard work, a number
of environment variables are set when PL/Rust executes `cargo`.

These are not environment variables that need to set manually.  Generally, these are auto-detected and cannot be 
overridden through configuration.

| Name                                        | Value                                                                         | How it's Used                                                                                                                                                                                                         |
|---------------------------------------------|-------------------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| PATH                                        | `~/cargo/bin:/usr/bin` or `/usr/bin` if "postgres" user has no home directory | The `PATH` environment variable is **only** set by PL/Rust if it detects that one isn't already set.  <br/>As mentioned above, this one *can* be overridden via the `plrust.PATH_override` GUC in `postgresql.conf`.  |
| RUSTC                                       | `plrustc`                                                                     | This is set to plrust's "rust driver" executable, named `plrustc`.  It must be on the system PATH.                                                                                                                    | 
| RUSTFLAGS                                   | `"-Clink-args=-Wl,-undefined,dynamic_lookup"`                                 | Used by `rustc` to indicate that Postgres internal symbols are only available at run-time, not compile-time.                                                                                                          |
| CARGO_TARGET_DIR                            | value of GUC `plrust.work_dir`/`target`                                       | This is the filesystem path `cargo` will store its intermediate compilation artifacts.                                                                                                                                |
 | CARGO_TARGET_X86_64_LINKER                  | `x86_64-linux-gnu-gcc`                                                        | Used only when cross-compiling *to* x86_64, this tells `rustc` which linker to use.  The `plrust.x86_64_linker` GUC can override the default.                                                                         |
| CARGO_TARGET_AARCH64_LINKER                 | `aarch64-linux-gnu-gcc`                                                       | Used only when cross-compiling *to* aarch64, this tells `rustc` which linker to use.  The `plrust.aarch64_linker` GUC can override the default.                                                                       |
 | PGX_TARGET_INFO_PATH_PG${MAJOR_VERSION_NUM} | unset unless `plrust.{x86_64/aarch64}_pgx_bindings_path` GUC is set           | Used only when cross-compiling *to* the specified target.  This tells `pgx` where to find the generated Postgres bindings for that platform.                                                                          | 
| PGX_PG_CONFIG_AS_EN_VAR                     | `true`                                                                        | Indicates to the `trusted-pgx` dependency, and ultimately `pgx` itself that instead of getting the values it needs for compilation from the Postgres `pg_config` tool, it should get them from environment variables. |
| PGX_PG_CONFIG_VERSION                       | Provided by the running Postgres instance                                     | Used by `pgx` to build the PL/Rust user function.                                                                                                                                                                     |
| PGX_PG_CONFIG_CPPFLAGS                      | Provided by the running Postgres instance                                     | Used by `pgx` to build the PL/Rust user function (technically unused by PL/Rust's build process as PL/Rust does not include the pgx "cshim" for which this is normally used).                                         |
| PGX_PG_CONFIG_INCLUDEDIR-SERVER             | Provided by the running Postgres instance                                     | Used by `pgx` to build the PL/Rust user function.                                                                                                                                                                     |

## Safety

Note that PL/Rust uses Rust's [`std::process::Command`](https://doc.rust-lang.org/beta/std/process/struct.Command.html) 
to exec `cargo`.  As such, it **will** inherit **all** environment variables set under the active backend `postgres` 
process.  We recommend Postgres' execution environment be properly sanitized to your organizations requirements.

As a pre-emptive measure, PL/Rust proactively un-sets a few environment variables
that could negatively impact user function compilation.
These are generally things used by the `pgx` development team that are not
necessary for PL/Rust.

* `DOCS_RS`
* `PGX_BUILD_VERBOSE`
* `PGX_PG_SYS_GENERATE_BINDINGS_FOR_RELEASE`
* `CARGO_MANIFEST_DIR`
* `OUT_DIR`



## Reserved environment variables

There are a number of other `pg_config`-related environment variables that plrust sets.  These are not currently used,
but are reserved for future use, should they become necessary to build a user function:

* `PGX_PG_CONFIG_BINDIR`
* `PGX_PG_CONFIG_DOCDIR`
* `PGX_PG_CONFIG_HTMLDIR`
* `PGX_PG_CONFIG_INCLUDEDIR`
* `PGX_PG_CONFIG_PKGINCLUDEDIR`
* `PGX_PG_CONFIG_INCLUDEDIR-SERVER`
* `PGX_PG_CONFIG_LIBDIR`
* `PGX_PG_CONFIG_PKGLIBDIR`
* `PGX_PG_CONFIG_LOCALEDIR`
* `PGX_PG_CONFIG_MANDIR`
* `PGX_PG_CONFIG_SHAREDIR`
* `PGX_PG_CONFIG_SYSCONFDIR`
* `PGX_PG_CONFIG_PGXS`
* `PGX_PG_CONFIG_CONFIGURE`
* `PGX_PG_CONFIG_CC`
* `PGX_PG_CONFIG_CPPFLAGS`
* `PGX_PG_CONFIG_CFLAGS`
* `PGX_PG_CONFIG_CFLAGS_SL`
* `PGX_PG_CONFIG_LDFLAGS`
* `PGX_PG_CONFIG_LDFLAGS_EX`
* `PGX_PG_CONFIG_LDFLAGS_SL`
* `PGX_PG_CONFIG_LIBS`
* `PGX_PG_CONFIG_VERSION`


## Influencing PL/Rust Compilation

If set, PL/Rust will use the `PLRUST_TRUSTED_PGX_OVERRIDE` environment variable when PL/Rust itself is being compiled.
See the [Choosing a different `plrust-trusted-pgx` dependency at compile time](install-plrust.md) section for details.