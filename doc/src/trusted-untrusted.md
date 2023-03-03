# Trusted and Untrusted PL/Rust

Normally, PL/Rust is installed as a "trusted" programming language named `plrust`.
In this setup, certain Rust and `pgx` operations are disabled to preserve security.
In general, the operations that are restricted are those that interact with the environment.
This includes file handle operations, require, and use (for external modules).
There is no way to access internals of the database server process or to gain
OS-level access with the permissions of the server process, as a C function can do.
Thus, any unprivileged database user can be permitted to use this language.

Here is an example of a function that will not work because file system operations are not allowed for security reasons:

```
EXAMPLE COMING SOON
```

The creation of this function will fail as its use of a forbidden operation will be caught by the validator.

Sometimes it is desirable to write Rust functions that are not restricted.
To handle these cases, PL/Rust can also be installed as an "untrusted" language.
In this case the full Rust language is available including `unsafe` code.
See the [Development Installation](install-plrust-dev.md) for steps to installing
untrusted PL/Rust.

The writer of an untrusted PL/Rust function must take care that the function cannot be used to do anything unwanted, since it will be able to do anything that could be done by a user logged in as the database administrator. Note that the database system allows only database superusers to create functions in untrusted languages.

If the above function was created by a superuser using the untrusted `plrust`, execution would succeed.

