# Designing for Trust

## The goal
Nominally, to make PL/Rust exist: a dialect of Rust nested in SQL that constitutes a "trusted procedural language".

## The caveat
A major obstacle to making PL/Rust a trustworthy language is that Rust is not an intrinsically safe language.

Again, Rust is not an intrinsically safe language.

There are three major details to this:

1. Rust has not been formally verified to have all of the safety properties it intends to have. Bugs exist that undoubtedly violate its own design for memory safety. These bugs will eventually be fixed, because there is no soundness bug that is considered a "breaking change", or rather, Rust considers all flaws in its type system that would prevent the type system from verifying memory safety to be acceptable to change and they are explicitly not governed by any stability promises. Nonetheless, Rust is only as safe as its implementation is safe.
2. Rust is split into two sublanguages: Safe Rust and Unsafe Rust. Most Rust is Safe Rust. An `unsafe { }` block allows the usage of Unsafe Rust code, and most Unsafe Rust code item declarations are also annotated with `unsafe`[1]. It is required to have Unsafe Rust as an implementation primitive in order to be able to specify the behavior of Rust: otherwise it would have to be written in another, also memory-unsafe language. By using both as part of Rust, certain guarantees based in the type system can traverse between Safe and Unsafe Rust and remain intact. Otherwise, the work to prove the type soundness would have to begin entirely within Safe Rust, without the ability to incrementally validate claims.
3. Rust is not safe against all logic errors, nor does it consider all operations to be `unsafe` that the programmer might think of as `unsafe`. For instance, Rust considers `panic!` to be "safe": arguably, it is very not safe for _someone_ if Rust code forms the core of an actively-running flight system for some airplane or helicopter and an uncaught panic terminates the flight system abruptly, rendering it inoperative for sufficiently long that the flight system cannot recover stability even after it reboots. It is also usually considered safe to perform IO on arbitrary files, but a database might take a dim view of writing to its storage files.

This three-part caveat, one might notice, is largely a problem of _definition_:

1. Safe according to whom?
2. Safe for what uses?
3. Safe in which context?

However, each of these remain distinct issues.

### Trusting with Insufficient Paranoia

The caveats that apply to Rust apply in very similar form to other existing procedural languages, whether or not they are "trusted":
1. The question is not whether there is another CVE to discover in PL/Perl or PL/pgSQL, it's how long it will take to find it.
2. The trusted languages have an underlying implementation in a memory-unsafe language. This poses the question of whether those languages are fully secured against the surface implementation being used to achieve unsafe effects.
3. Some undesirable effects can still be achieved via the procedural languages. Notably, it's not clear they have much of a defense against e.g. using infinite loops to lock up that thread of execution rather than proceed further.

This is not to say these languages are equally safe or unsafe. It merely is to observe that in the presence of sufficient paranoia, all implementations for procedural languages that currently exist are somewhat hard to trust. Any trusted language still means allowing arbitrary users with access to the database to execute essentially arbitrary code within that database. If there is a weak point and they care enough to keep searching, then they will find it.

## Safety and trust are implementation-defined

Rust defines "safety" around the concept of "memory safety", and uses a type system that includes ownership types to implement that.
Postgres defines "trusted" around the concept of limiting the code to things that are of either no consequence to the database or that the database was going to allow a user to hypothetically do anyway, and uses access control and SQL roles to implement that.

## Solving the problems

A perfectly elegant solution would address all of these parts of the problem in one swoop.
However, that would require there to be some unifying dilemma that, if answered, that can easily handle all of these outward projections.
Unfortunately, a formally-verified wasm virtual machine that can be used to safely execute arbitrary Rust code inside it,
yet still bind easily against PostgreSQL's C API is... a tall order. In other words, the more elegant solution simply doesn't exist yet.
Because it doesn't exist, it's debatable if it would actually elegantly solve the issue, as we can't actually assess that claim.

In this, more clumsy world, such a three-part problem calls for a three-part solution... at least.

3. To align Safe Rust more closely with what Postgres expects a trusted language to be able to do, replace `std` with `postgrestd`.
2. To prevent Unsafe Rust from being used to violate expectations, bar the use of `unsafe` code.
1. Deploy any and all additional hardening necessary.
0. Keep doing that, actually: Defense in depth is a good thing.

### Safety, Unwinding, and `impl Drop`

In Rust, the `Drop` trait promises that if execution reaches certain points in a program then a destructor has been run.
There is an immediate and obvious problem with this: Rust does not guarantee forward progress and includes diverging control flow.
In other words, it is possible for Rust to never reach certain points in control flow.

In a more targeted fashion, it is possible also to `mem::forget` something with `Drop`, or to wrap it in `ManuallyDrop`.
Together, these facts mean that a destructor can never be relied on to be run when following arbitrary control flow.
Only Rust control flow that lacks these features can be expected to run all destructors.

This raises the question: what are destructors "for", if code can't actually rely on them to do anything?
In Rust, `Drop` arguably primarily exists to dismantle objects so they are not left in invalid states, and to allow reclaiming resources early.
The first only matters if the invalid state can potentially be reached later in the program, a destructor is only relevant if the program continues.
Otherwise, it usually takes unsound code to observe states in something that has been subject to `mem::forget`, as that function consumes the object in question.
Everything moved into it should be "gone" and no longer visible to the rest of the program state unless `unsafe` has been used.
The second only matters if the resources are not reclaimed by some other mechanism, such as e.g. process termination.

Thus, if an error inside a function causes Rust to leave behind all of the code that contains broken state, then at least hypothetically not running any destructors in that code whatsoever, even if it does not include `mem::forget`, is "not a problem". This implies one possible error handling strategy for PL/Rust, rather than trying to handle errors from Postgres, is simple abject submission that terminates the function instantly. This is a somewhat crude approach as it also means errors from Postgres that plausible could be possible to handle soundly in PL/Rust are not, but it also prevents having to handle miscellaneous other edge-cases correctly.

### Further possible defense in depth

#### Dynamic allocator hardening?

[1]: There are a few cases where Unsafe Rust code can be declared without it being visibly denoted as such, and these are intended to be phased out eventually, but in these cases they generally still require an `unsafe { }` block to be called or they must be wrapped in an `unsafe fn`. The absence of the `unsafe` token can only be bypassed in Rust by declaring an `extern fn` (which is implicitly also an `unsafe fn`, allowing one to fill it with other `unsafe` code) and then calling that function from another language, like C.
