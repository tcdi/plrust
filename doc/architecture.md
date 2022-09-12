# Architecture

`plrust`, a rust based [Postgres Extension for Postgres](https://github.com/tcdi/pgx), provides a rust language handler for Postgres Functions. The installed language handler compiles postgres function, written in rust, executes the function <where>. 

## Trusted Language

In order to create a [trusted](https://www.postgresql.org/docs/current/sql-createlanguage.html) language handler in Postgres we must restrict the language handler from specific operations such as 

- File Handle Operations
- cannot access internals of the database server process
- cannot gain OS level access with the permissions of the server process.
- Any unprivileged database user can be permitted to use the language ([postgresql.org](https://www.postgresql.org/docs/current/plperl-trusted.html))

## Rust

While interpreted languages like Perl or Python execute in a runtime component, the Rust language must be compiled to a specific target.  A target can be a shared object, binary, or wasm artifact.  The operating system or host process executes the shard object (or binary) in a similar manner to C.  Rust provides support for unsafe memory operations through the use of the unsafe keyword.  Code tagged in this manner indicates to the compiler that the code does not need to be checked for memory safety and the developer has ensured it safety.

## Design Goals

Design a custom rust compilation target for Postgres that provides nearly "safe" (as Rust defines it) and "trusted" (as Postgres defines a procedural language) plrust.

The goals for the approach include

* Architecture support for x86_64 and aarch64
* Operating system support for Linux
* Disallow File Handle operations
* Disallow access to the internals of the database
* Disallow access to the OS as the user executing the Postgres process 
* Disallow access into active postmaster process, i.e. no ability to reach into Postgres memory space, despite executing inside it.
* Gracefully handle rust panics and have them interoperating with Postgres' transaction system
* Memory allocation within Postgres' pallow/pfree functions

## Approach

The plrust extension is compiled using the standard rust library.  The postgrestd library is used to compile the functions written using PLRust.  The postgrestd library is a libraries are compiled using The 

Following an approach similar to the selection between libc and the musl standard library for compilation, a Postgres compilation target is defined that instructs the compiler to use the postgrestd library.  The postgrestd library provides the rust standard library interfaces except in the case where it is desirable to prevent access.  In those cases the code is [configured](https://doc.rust-lang.org/stable/rust-by-example/attribute/cfg.html) to be not present. The result is a small shim on top of the rust library limited access to the standard library.


## Bird's Eye View

![](assets/architecture_1.png)


## Code Map

This section talks briefly about various important directories and data structures.

### pallocator

The Postgres allocator project maps the Postgres memory allocation methods to standard library methods.  the memory allocation from standard library methods to postgres specific methods.

alloc -> palloc
free -> pfree
realloc - prealloc

### postpanic

The Postgres panic project maps the rust panic to the postgres panic, allowing Postgres to handle the panic within the transaction system.

### std


## Cross-Cutting Concerns

This sections talks about the things which are everywhere and nowhere in particular.

### Code generation

### Cancellation

### Testing

### Error Handling

### Observability
