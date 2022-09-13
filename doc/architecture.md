# Architecture

`plrust`, a rust based [Postgres Extension for Postgres](https://github.com/tcdi/pgx), provides a rust language handler for Postgres Functions. The installed language handler compiles postgres function, written in rust, executes the function <where>. 

## Trusted Language

In order to create a [trusted](https://www.postgresql.org/docs/current/sql-createlanguage.html) language handler in Postgres we must restrict the language handler from specific operations such as 

- File Handle Operations
- Limit access to the database to that of other procedural language functions
- Limit access to system resources to those of a trusted language user function
- Any unprivileged database user can be permitted to use the language ([postgresql.org](https://www.postgresql.org/docs/current/plperl-trusted.html))

## Rust

A target tuple describes a "platform" that can execute code. Rust uses rustc, which requires that code is ahead-of-time compiled in order to do code generation, so it requires a target tuple that defines the code object it must generate. A code object has a format (e.g. ELF or Windows PE) which an operating system supports, instructions (e.g. aarch64 or wasm) which a machine architecture supports, and calls to system interfaces to the operating system (such as via GNU `libc.so` or MacOS `libSystem.dylib`) which require holistic support. These code objects may be executables (which the system may initialize as a process) or libraries (which may be "linked", relocating code from them into the final executable at build time, or loading their code to call at runtime). Libraries used at build time are also called static libraries, objects, or archives. Libraries used at runtime are also called dynamic libraries or shared objects.

The Rust compiler builds the Rust standard library as a static library and then links it into Rust code objects. The contents of this static library include the code which dynamically links against system interfaces. These system interfaces are what postgrestd intercepts by itself being a modification of the Rust standard library.

The extension called "PL/Rust" which includes the language handler is responsible for covering the linking and loading steps. This extension may have privileges that user functions do not, using the Rust std of the same host target that PostgreSQL itself is compiled for, to interoperate in that privileged mode. This is as-usual for language handlers: they must typically be written in C.

## Design Goals

Design a custom rust compilation target for Postgres that provides nearly "safe" (as Rust defines it) and "trusted" (as Postgres defines a procedural language) plrust.

The goals for the approach include

* Architecture support for x86_64 and aarch64
* Operating system support for Linux
* Disallow File Handle operations
* Disallow access to the internals of the database
* Disallow access to the OS as the user executing the Postgres process 
* Disallow access into active postmaster process, i.e. no ability to reach into Postgres memory space, despite executing inside it.
* Gracefully handle rust panics and have them interoperate with Postgres' transaction system
* Memory allocation within Postgres' palloc/pfree functions

## Approach

The plrust extension is compiled using the standard rust library.  The postgrestd library is used to compile the functions written using PLRust.  The postgrestd library is a libraries are compiled using The 

Following an approach similar to the selection between libc and the musl libc standard library for compilation, a Postgres compilation target is defined that instructs the compiler to use the postgrestd library.  The postgrestd library provides the rust standard library interfaces except in the case where it is desirable to prevent access.  In those cases the code is [configured](https://doc.rust-lang.org/stable/rust-by-example/attribute/cfg.html) to be not present. The result is a small shim on top of the rust library limited access to the standard library.


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
