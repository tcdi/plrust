# Designing for Trust

## A note on definitions
In order to allow building a Postgres extension in Rust, quite a lot of bindings to C code are required,
and a language handler is necessarily a Postgres "C" function, which is usually packaged as a Postgres extension.
Since the only Postgres extension of true concern for PL/Rust is the language handler and associated components,
and most other functions that will be embedded in Postgres will be managed by this extension, I will redefine the difference arbitrarily for this document: the language handler is a Postgres extension, and "Postgres function" will be used to refer to any Postgres function _except_ a language handler.

## The goal
Nominally, to make PL/Rust exist: a dialect of Rust nested in SQL that can function as a "trusted procedural language".

## The caveat
A major obstacle to making PL/Rust a trustworthy language is that Rust is not an intrinsically safe language.

Again, Rust is not an intrinsically safe language.

There are three major details to this:

1. Rust has not been formally verified to have all of the safety properties it intends to have. Bugs exist that undoubtedly violate its own design for memory safety. These bugs will eventually be fixed, because there is no soundness bug that is considered a "breaking change", or rather, Rust considers all flaws in its type system that would prevent the type system from verifying memory safety to be acceptable to change and they are explicitly not governed by any stability promises. Nonetheless, Rust is only as safe as its implementation is safe.
2. Rust is split into two sublanguages: Safe Rust and Unsafe Rust. Most Rust is Safe Rust. An `unsafe { }` block allows the usage of Unsafe Rust code, and most Unsafe Rust code item declarations are also annotated with `unsafe`[^1]. It is required to have Unsafe Rust as an implementation primitive in order to be able to specify the behavior of Rust: otherwise it would have to be written in another, also memory-unsafe language. By using both as part of Rust, certain guarantees based in the type system can traverse between Safe and Unsafe Rust and remain intact. Otherwise, the work to prove the type soundness would have to begin entirely within Safe Rust, without the ability to incrementally validate claims. However, this means that Unsafe Rust is always waiting behind all Safe Rust, so the abstraction boundary must be evaluated carefully.
3. Rust is not safe against all logic errors, nor does it consider all operations to be `unsafe` that the programmer might think of as `unsafe`. For instance, Rust considers `panic!` to be "safe": arguably, it is very not safe for _someone_ if Rust code forms the core of an actively-running flight system for some airplane or helicopter and an uncaught panic terminates the flight system abruptly, rendering it inoperative for sufficiently long that the flight system cannot recover stability even after it reboots. It is also usually considered safe to perform IO on arbitrary files, but a database might take a dim view of writing to its storage files.

This three-part caveat, one might notice, is largely a problem of _definition_:

1. Safe according to whom?
2. Safe for what uses?
3. Safe in which context?

However, each of these remain distinct issues because they cover different domains: validity, implementation, and context.

### Is Trust Insufficient Paranoia?

The caveats that apply to Rust apply in very similar form to other existing procedural languages, whether or not they are "trusted":
1. The question is not whether there is another vulnerability to discover in PL/Tcl, PL/Perl, PL/pgSQL, or with their shared interface with PostgreSQL: it's how long it will take to find it, whether anyone bothers to look, and whether it can actually be used to inflict damage.
2. The trusted procedural languages have an underlying implementation in a memory-unsafe language. This poses the question of whether those languages are fully secure against the surface implementation being used to achieve unsafe effects. They undoubtedly are against trivial attacks.
3. Some undesirable effects can still be achieved via the procedural languages. Notably, it's not clear they have much of a defense against e.g. using infinite loops to lock up that thread of execution rather than proceed further.

This is not to say these languages are equally safe or unsafe: there's some advantages in being able to deploy dynamic checks.
It merely is to observe that in the presence of sufficient paranoia, all implementations for all languages that exist are hard to trust.
Web browsers face similar dilemmas, and many users run browsers with JavaScript limited or disabled because they do not trust it, despite its sandboxing.
Any trusted language still means allowing arbitrary users with access to the database to execute code within that database which has broad capabilities.
If there is a weak point those capabilities can be applied to break through, and an attacker cares enough to keep searching, it will be found.

In effect, "trust" in practice only exists in two cases:
- not being aware of the consequences
- being willing to accept the possibility that worst-case consequences might happen

## Safety and trust are implementation-defined

Rust defines "safety" around the concept of "memory safety", and uses a type system that includes ownership types to implement that.

For PostgreSQL's database code, a "trusted procedural language" has only one concrete definition:
Did a database administrator install it with the TRUSTED designation?
There's nothing technically stopping a DBA with the appropriate privileges from installing an "untrusted" language as TRUSTED in PostgreSQL.

A more amorphous but more practically useful definition is extensively implied throughout the documentation on procedural language:
A trusted procedural language is a language where, if you install it as TRUSTED, this decision will not immediately bite you on the ass.
The Postgres documentation defines this kind of "trusted" around the idea of limiting trusted language code to effects that either
are of no consequence to the database or that the database was going to allow a user to hypothetically do anyway,
and it uses dynamic checks and SQL roles to assist implementing that.
Specifically, this means a trusted language's code should also respect SQL roles and not produce unintentional denials of service.
It may still serve as an attack vector on the system, as can normal SQL-DDL commands, but if it does,
it should make it slightly more frustrating for an attacker than running arbitrary assembly (AKA shellcode) would permit.
Many attacks of this nature unfortunately will still end in being able to run shellcode if successful.

It may be worth drawing a parallel to cryptography, another way of assuring data security and integrity:
many supposedly "one-way" hash functions can theoretically be reversed by an attacker with sufficient power.
The security of hashed data usually instead lies in making it so that the attacker would require large amounts of computational power,
considerable time, and probably at least one or two novel breakthroughs in the understanding of computation itself,
or else they may be spending so much time that the Earth will grow cold before they can unlock the data.
Or hopefully at least a few days, allowing time for, say, discovering the breach and generating new passwords.
We call something that achieves this goal "secure", even though in actuality it is in fact "eventually breakable".
Likewise, a "trusted procedural language" will in practice be "eventually breakable",
and the goal is not necessarily to be inviolate but to offer some resistance.

A quality implementation of a trusted procedural language should offer enough resistance that you can worry much less.
The rest of this discussion will revolve around what is ultimately a proposal to implement PL/Rust
as a high-quality trusted procedural language and how to evaluate that as an ongoing event,
rather than one that is necessarily expected to be "finished".

## Solving the problems

A perfectly elegant solution would address all of these parts of the problem in one swoop.
However, that would require there to be some unifying dilemma that, if answered, can easily handle all of these outward projections.
Unfortunately, a formally-verified wasm virtual machine that can be used to safely execute arbitrary Rust code inside it,
yet still bind easily against PostgreSQL's C API is... a tall order. In other words, the more elegant solution simply doesn't exist yet.
Because it doesn't exist, it's debatable if it would actually elegantly solve the issue, as we can't actually assess that claim.
Notably, it's not clear that allowing arbitrary bindings in such a wasm sandbox would not simply create a sandbox that can do dangerous things.
A protective box that encloses its contents yet still has many dangerous projections outside it is usually called a "tank",
and is considered to be a weapon of war, which may not be something you wish to introduce into your database.

So in this, more clumsy world, such a three-part problem calls for a three-part solution... at least.

1. To align Safe Rust more closely with what Postgres expects a trusted language to be able to do, replace `std` with `postgrestd`.
2. To prevent Unsafe Rust from being used to violate expectations, bar the use of `unsafe` code.
3. Deploy any and all additional hardening necessary.
4. Keep doing that, actually: Defense in depth is a good thing.

Eventually, using more effective and total layers of sandboxing can be used when that becomes more convenient, but the problem would remain:
Normally, Rust code has the ability to call bindings that can do things a trusted procedural language should not be allowed to do,
so if you allow Rust to bind calls to arbitrary external functions into wasm, then you allow Rust to "break trust".
A comprehensive approach that blocks off these exit routes is still required, and any additional sandboxing serves as reinforcement.

### Safety, Unwinding, and `impl Drop`

<details>
<summary>
Needs rewrite after rewrite of PGX error handling
</summary>

In Rust, the `Drop` trait promises that if execution reaches certain points in a program then a destructor has been run.
There is an immediate and obvious problem with this: Rust does not guarantee forward progress and includes diverging control flow that "never returns".
Thus it is possible for Rust code to never reach certain points in control flow, such as by invoking `panic!()` first.
Normally, however, `panic!()` will cause "unwinding", which walks back through Rust code to the nearest `catch_unwind`, running `Drop` as it goes.

However, this is not always the case, and `panic!()` may be implemented by other forms of divergence such as immediate termination.
This may seem surprising, but it is a simple extension of the natural observation that `SIGKILL` exists,
or its sundry equivalents on non-Unix-like operating systems, and Rust code usually runs under an operating system.
Rust does not consider terminating Rust code to be a violation of memory safety, because ceasing to progress
is considered the appropriate way to respond to a situation where the program is not capable of soundly handling further events.
A possible event that can cause this is the "panic-in-panic" scenario: if unwinding also causes a panic, Rust simply aborts.

In a more targeted fashion, it is possible also to `mem::forget` something with `Drop`, or to wrap it in `ManuallyDrop`.
Together, these facts mean that a destructor can never be relied on to be run when following arbitrary control flow.
Only Rust control flow that lacks these features can be expected to run all destructors.
In other words: `Drop` can be intercepted by both events inside normal Rust code and also "outside" it.

</details>

<!--
need to discuss:
- statics
- unwind runtimes
- landing pads and where they are located in PL/Rust contexts
- maybe thread safety?
- other stuff with palloc?
-->

### Controlling `unsafe`

Code can by hypothetically verified to be "safe" by either scanning the tokens directly using a procedural macro or by compiling it with various lints of the Rust compiler to detect and constrain use of `unsafe` enabled.

#### Is automatically blocking all `unsafe` code enough?

No.

The problem with blocking all `unsafe` code is that pgx, the Rust standard library, and essentially all implementation details of PL/Rust,
will be implemented using `unsafe` code. There are also many crates which are soundly implemented and theoretically fine to use for PL/Rust,
but rely on an `unsafe` implementation primitive.

Further, some way must exist to implement the function call interface from PostgreSQL to Rust code.
In PL/Rust, that is done via the [pgx crate][pgx@crates.io]. This requires a lot of `unsafe` code.
Thus, in order to compile any PL/Rust function, *a lot of unsafe code must be used*.
This also means that something must be done to prevent the use of pgx's `unsafe fn` in PL/Rust
while still allowing pgx to use `unsafe` code to implement its own interfaces.

#### plutonium


### `postgrestd`: containing the problem

If Rust is not allowed to bind against arbitrary external interfaces, then it only has `std` and whatever crates are permitted.
This makes controlling `std` a priority, and `postgrestd` is used to implement that.

The result of this is that as long as only Rust code compiled with the `postgrestd` fork is executed via PL/Rust,
and as long as e.g. arbitrary `unsafe asm!` is not permitted, an escalation in privileges
cannot simply jump outside the database and start doing arbitrary things.
It is limited to subverting the database, which admittedly is still a bountiful target,
but in this event containing the database itself can still be meaningfully done.

### The other elephant in the room: pgx

In addition to being used as the implementation detail of PL/Rust, pgx offers a full-fledged interface for building Postgres extensions in general.
This means that like the Rust standard library, pgx is not perfectly adapted to being an interface for a trusted procedural language.
There are two possible options in carving out what parts of pgx are appropriate to use:

- remove all inappropriate features behind `#[cfg]` blocks, OR
- create a separate crate and expose it as the pgx-Postgres user-callable interface

Neither of these are perfectly satisfying because neither option provides a neatly-defined, automatic answer
to the question "of pgx's safe code, what should be allowed?" to begin with.

There is also the unfortunate question of "is pgx's safe code actually sound?"
The crate's early implementation days included a few declared-safe wrappers that didn't fully check all invariants,
and in some cases did not document the implied invariants, so an [audit of code in pgx][issue-audit-c-calls] is required.
There is no getting around this, as it falls back on the fundamental problem of all procedural languages:
They can only be as trustworthy as their implementations, which puts a burden on their implementation details to be correct.
Fortunately, most of this audit has already been accomplished simply by the crate receiving scrutiny over the past 3 years.

### Further defense in depth: Heap attacks?

When you allow a user to run code in your database's process, you are allowing them to attempt to subvert that process,
so all users to some extent must _also_ be trusted with the tools you are giving them,
claims that trusted procedural languages allow untrusted users to run untrusted code besides. They just can be trusted _less_.
However, if a user is expected to possibly "sublet" their tenancy to another user, creating a complex multitenancy situation,
where the current superuser adopts the position of a "hyperuser", and the user adopts the position of "virtual superuser",
the hyperuser who decides what languages are installed may still want to allow the virtual superuser's guests to run code,
but has to be aware that they have _even less_ trust.
This means various traditional attack venues, e.g. heap attacks, become even more of a concern,
as the hyperuser may have to mount a defense against the virtual superuser's guests,
and the virtual superuser may install and run PL/Rust code on behalf of these guests.

These are possible future directions in adding layers of security, not currently implemented or experimented with yet.

#### Dynamic allocator hardening?

While PL/Rust merely interposes palloc, it... still interposes palloc. This means it can implement a "buddy allocator".
Since it's possible to control the global allocator for Rust code, this can help interfere with attacks on the heap.
This is likely necessary, at the cost of some runtime overhead (offset by PL/Rust precompiling code for execution speed),
to buy security against any attacks that target flaws in the Rust type system when those issues are not solved.
Having to do this to harden a "memory-safe" language is not unusual, and the system administrator
should be aware of this when deploying PostgreSQL and consider deploying PostgreSQL with a similarly hardened allocator
so that all allocations benefit from this protection, but it's not unreasonable to want a second layer for PL/Rust.

#### Background worker executor?

The process boundary offers a great deal of resilience against heap attacks. Background workers are separate processes, and
PL/Java implementations use a similar approach of running code inside a daemon (which also takes care of compiling code).
This may trade off a lot of performance gains from PL/Rust's overall approach, but it still may be worth it.

# Notes

[^1]: There are a few cases where Unsafe Rust code can be declared without it being visibly denoted as such, and these are intended to be phased out eventually, but in these cases they generally still require an `unsafe { }` block to be called or they must be wrapped in an `unsafe fn`. The absence of the `unsafe` token can only be bypassed in Rust by declaring an `extern fn` (which is implicitly also an `unsafe fn`, allowing one to fill it with other `unsafe` code) and then calling that function from another language, like C.

[pgx@crates.io]: https://crates.io/crates/pgx/
[issue-audit-c-calls]: https://github.com/tcdi/pgx/issues/843
