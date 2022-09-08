# Architecture

`plrust`, a rust based [Postgres Extension for Postgres](https://github.com/tcdi/pgx), provides a rust language handler for Postgres Functions. The installed language handler compiles postgres function, written in rust, executes the function <where>. 

## Trusted Language

In order to create a [trusted](https://www.postgresql.org/docs/current/sql-createlanguage.html) language handler in Postgres we must restrict the language handler from specific operations such as 

- File Handle Operations
- cannot access internals of the database server process
- cannot gain OS level access with the permissions of the server process.
- Any unprivileged database user can be permitted to use the language ([postgresql.org](https://www.postgresql.org/docs/current/plperl-trusted.html))

## Design Goals

Design a custom rust compilation target for Postgres that provides nearly "safe" (as Rust defines it) and "trusted" (as Postgres defines a procedural language) plrust.

The goals for the approach include

* Architecture support for x86_64 and aarch64
* Operating system support for Linux
* Disallow File Handle operations
* Disallow access to the internals of the 
* Disallow access to the OS using the postmaster 
* Disallow access into active postmaster process, i.e. no ability to reach into Postgres memory space, despite executing inside it.
* Gracefully handle rust panics and have them interoperatin with Postgres' transaction system
* Memory allocation within Postgres' pallow/pfree functions
* 

## Approach

The plrust extension is compilied using the standard rust library.  The postgrestd library is used to compile the functions written using PLRust.  The Postgrestd library is a libraries are compilied using The 

## Bird's Eye View

![](assets/architecture_1.png)


## Code Map

This section talks briefly about various important directories and data structures.

###

## Cross-Cutting Concerns

This sections talks about the things which are everywhere and nowhere in particular.

### Code generation

### Cancellation

### Testing

### Error Handling

### Observability
