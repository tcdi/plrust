# External Dependencies

PL/Rust supports the use of external dependencies. By default, this is unrestricted even when PL/Rust is used as a 
Trusted Language Handler, allowing user functions to specify any desired dependency.

For instance:

```sql
CREATE OR REPLACE FUNCTION randint() RETURNS bigint LANGUAGE plrust AS $$
[dependencies]
rand = "0.8"

[code]
use rand::Rng; 
Ok(Some(rand::thread_rng().gen())) 
$$;
```

It is recommended that administrators create a dependency allow-list file and specify its path in `postgresql.conf` using 
the `plrust.allowed_dependencies` setting.

To disable external dependencies completely, create a zero-byte file or point the configuration to `/dev/null`.

## The Allow-List File

The dependency allow-list is a TOML file. Its format mirrors that of the `[dependencies]` section in a standard 
`Cargo.toml`, albeit with certain requirements on the version strings.

### The Format

The file consists of `dependency_name = version_requirement` pairs, where `version_requirement` can adopt several forms. 
It can be a quoted string such as `"=1.2.3"`, a TOML table like `{ version = "=1.2.3", features = ["a", "b", "c"] }`, or
an array of either, such as `[ "=1.2.3", { version = "=1.2.3" }, ">=4, <5"`.

Here is a valid allow-list file for reference:

```toml
rand = ">=0.8, <0.9"
bitvec = [">=1, <2", "=0.2", { version = "1.0.1", features = [ "alloc" ], default-features = false }]
```

This added flexibility empowers administrators to specify the exact crate version and its associated features and properties.

When a `LANGUAGE plrust` function designates a dependency and version, the largest (presumably most recent) matching 
version from the allow-list is used.

### Version Requirement Format

PL/Rust employs Cargo's interpretation of semver to manage dependency versions, but it requires each version requirement
to be an exact value like `=1.2.3`, a bounded range such as `>=1, <2`, or a bare wildcard (`*`).

For example, these are valid version requirement values:

```toml
rand = "=0.8.5"
serde = ">=1.0.151, <1.1"
bitvec = "*"
```

These, however, are not:

```toml
rand = "0.8.5"
serde = ">1.1"
```

The `cargo` tool may select a slightly different version based on the specification. However, with exact and bounded 
values, `cargo`'s choices are limited to the versions that administrators allow.

The bare wildcard pattern (`*`) is acceptable and has a unique interpretation within a user `LANGUAGE plrust` function.

### Using a Dependency

As shown above, a `LANGUAGE plrust` function can include a `[dependencies]` section. Authors should specify exact versions
for each dependency. PL/Rust will match this exact version with an entry in the allow-list.

If a function requests a version in the `1.2.3` format and it matches an entry on the allow-list, PL/Rust will revise
it to an exact version, i.e., `=1.2.3`.

If the allow-list merely contains a wildcard version:

```toml
rand = "*"
```

... and the user function asks for a specific version, such as `0.8.5`, PL/Rust will utilize that exact version.

Conversely, if the allow-list specifies one or more particular version requirements...

```toml
rand = [ "0.8.5", "0.6" ]
```

... and the PL/Rust function requests a wildcard (i.e., `rand = "*"`), PL/Rust will select the largest version requirement
from the allow-list. In this case, it would be `0.8.5`.

### Working with Crate Features

When a user function employs a crate from the allow-list, the allow-list controls the permitted set of dependency properties 
such as `features` and `default-features` for each version. Users cannot override these. They can specify them, but the
specifications must match exactly with the allow-list.

This control enables administrators to dictate the usage of dependencies.

For instance, this would be acceptable for a user function:

```sql
CREATE OR REPLACE FUNCTION randint(seed bigint) RETURNS bigint STRICT LANGUAGE plrust AS $$
[dependencies]
rand = { version = "*", features = [ "small_rng" ], default-features = false }

[code]
use rand::rngs::SmallRng;
use rand::SeedableRng;
use rand::RngCore;

let mut rng = SmallRng::seed_from_u64(seed as _);
Ok(Some(rng.next_u64() as _))
$$;
```

Provided that the allow-list includes the following:

```toml
rand = { version = "=0.8.5", features = [ "small_rng" ], default-features = false }
```

Note that the user function could omit the dependency features since the allow-list declares them:

```sql
CREATE OR REPLACE FUNCTION randint(seed bigint) RETURNS bigint STRICT LANGUAGE plrust AS $$
[dependencies]
rand = "*"

[code]
use rand::rngs::SmallRng;
use rand::SeedableRng;
use rand::RngCore;

let mut rng = SmallRng::seed_from_u64(seed as _);
Ok(Some(rng.next_u64() as _))
$$;
```

### Operational Notes

- The dependency allow-list file path must be set in `plrust.allowed_dependencies` GUC value in `postgresql.conf`.
- Changing the GUC value requires a configuration reload on the database to take effect.
- The file must be readable by the user that runs Postgres backend connections. Typically, this user is named `postgres`.
- Every time a `CREATE FUNCTION ... LANGUAGE plrust` statement is executed, the file is read, parsed, and validated. This arrangement allows administrators to edit it without needing to restart the Postgres cluster.
