# External Dependencies

PL/Rust supports using external dependencies.  Out of the box, even as a Trusted Language Handler, this is unrestricted.
A user function may specify any dependency they wish.

For example:

```sql
CREATE OR REPLACE FUNCTION randint() RETURNS bigint LANGUAGE plrust AS $$
[dependencies]
rand = "0.8"

[code]
use rand::Rng; 
Ok(Some(rand::thread_rng().gen())) 
$$;
```

It is suggested that administrators create a dependency allow-list file and configure its path in `postgresql.conf` with
the `plrust.allowed_dependencies` setting.

If external dependencies are not wanted at all, create a zero-byte file or point the configuration to `/dev/null`.

## Allow-list Format

PL/Rust's dependency allow-list is a TOML file whose format is akin to a standard `Cargo.toml`'s `[dependencies]` section.
The format is a little different, however, and imposes some requirements on the version strings specified.

### The Format

The file format is a list of `dependency_name = version_requirement` pairs, however `version_requirement` can take a few different
forms.  It can be a quoted string, such as `"=1.2.3"`, a TOML table, such as `{ version = "=1.2.3", features = ["a", "b", "c"] } }`,
or an array of either of those, such as `[ "=1.2.3", { version = "=1.2.3" }, ">=4, <5"`.

As an example, this is a valid allow-list file:

```toml
rand = ">=0.8, <0.9"
bitvec = [">=1, <2", "=0.2", { version = "1.0.1", features = [ "alloc" ], default-features = false }]
```

The reason for this added flexibility to allow the administrator to declare the most precise crate version they wish
along with that version's specific set of features and other dependency properties.

When a `LANGUAGE plrust` function specifies a dependency and version, it finds the largest (presumably "most recent")
allowed version that matches what the user's plrust function requested.  More on this below.

### Version Requirement Format

PL/Rust uses Cargo's interpretation of semver to manage dependency versions, however PL/Rust dictates that each version
requirement must either be a single exact value such as `=1.2.3`, a bounded range such as `>=1, <2`, or a bare wildcard
(ie, `*`).

For example, these are valid version requirement values:

```toml
rand = "=0.8.5"
serde = ">=1.0.151, <1.1"
bitvec = "*"
```

These are not:

```toml
rand = "0.8.5"
serde = ">1.1"
```

The reason for this is that `cargo` is allowed to pick a slightly different version depending on how it was specified.
With exact or bounded values, `cargo` becomes limited to what the administrator has decided is allowed.

Using the bare wildcard pattern (`*`) is allowed and has a special meaning as it relates to a user `LANGUAGE plrust`
function.

### Using a Dependency

As demonstrated above, a `LANGUAGE plrust` function can have a `[dependencies]` section.  The function author is encouraged
to specify exact versions for each desired dependency, and PL/Rust will use that version if it is found to match one
of the allow-list entries for that dependency.

If the user requests a full, dotted triplet version such as `1.2.3` that is found to match one of the allow-list version
requirements, then PL/Rust will transparently rewrite it to be that exact version; it will become `=1.2.3`.

If the allow-list simply contains a wildcard version, for example:

```toml
rand = "*"
```

And the user function requests a non-wildcard version such as `0.8.5`, then PL/Rust will use that specific version.  If
the reverse is the case where the allow-list contains one or more specific version requirements, such as:

```toml
rand = [ "0.8.5", "0.6" ]
```

And the PL/Rust function requests a wildcard (ie, `rand = "*"`), then PL/Rust will choose the largest version requirement
from the allow-list.  In this case, that would be `0.8.5`.

### Working with Crate Features

When a user function uses an allow-list restricted crate, the allow-list controls, per version, the allowed set of 
dependency properties such as `features` and `default-features`.  A user function cannot override these.  It can specify
them, but they must exactly match what is found in the allow-list.

This restriction allows the administrator to have full control over how a dependency can be used.

For example, is fine for a user function:

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

As long as the allow-list contains the following:

```toml
rand = { version = "=0.8.5", features = [ "small_rng" ], default-features = false }
```

Note that since the allow-list declares the dependency features, the user function could have elided them:

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

### Operations Notes

- The dependency allow-list file must be configured in `postgresql.conf` where its full path is the value of the 
`plrust.allowed_dependencies` GUC.

- This file must be readable by the user that Postgres backend connections are run as.  Typically, the user named is named
`postgres`.

- The file is read, parsed, and validated every time a `CREATE FUNCTION ... LANGUAGE plrust` statement is executed.  Doing
so allows an administrator to modify it without requiring a Postgres cluster restart. 