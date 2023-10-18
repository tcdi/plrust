# Dynamic Function Calling

PL/Rust provides the ability to dynamically call any function (callable to the current user) directly from a Rust 
function.  These functions can be in *any* language, including `sql`, `plpgsql`, `plrust`, `plperl`, etc.

The call interface is dynamic in that the callee is resolved at runtime and its argument and return types are also 
checked at runtime.  While this does introduce a small bit of overhead, it's significantly less than doing
what might be the equivalent operation via Spi.

The ability to dynamically call functions enables users to write functions in the language that makes the most sense
for the operation being performed.  In many cases, a `LANGUAGE plpgsql` function is exactly what's needed, and a 
`LANGUAGE plrust` function can now use its result to execute further, possibly CPU-intensive, work.


## Important Rust Types

This dynamic calling interface introduces two new types that are used to facilitate dynamically calling functions:
`Arg` and `FnCallError`.

### `Arg`

`Arg` describes the style of a user-provided function argument.

```rust
/// The kinds of [`fn_call`] arguments.  
pub enum Arg<T> {
    /// The argument value is a SQL NULL
    Null,

    /// The argument's `DEFAULT` value should be used
    Default,

    /// Use this actual value
    Value(T),
}
```

Rust doesn't exactly have the concept of "NULL" nor does it have direct support for overloaded functions.  This is where
the `Null` and `Default` variants come in.

There's a sealed trait that corresponds to this enum named `FnCallArg`.  It is not a trait that users needs to implement,
but is used by PL/Rust to dynamically represent a set of heterogeneous argument types.

### `FnCallError`

There's also a set of runtime error conditions if function resolution fails.  These are recoverable errors in that user
code could `match` on the return value and potentially make different decisions, or just raise a panic with the error to
immediately abort the current transaction.

```rust
/// [`FnCallError`]s represent the set of conditions that could case [`fn_call()`] to fail in a
/// user-recoverable manner.
#[derive(thiserror::Error, Debug, Clone, Eq, PartialEq)]
pub enum FnCallError {
    #[error("Invalid identifier: `{0}`")]
    InvalidIdentifier(String),

    #[error("The specified function does not exist")]
    UndefinedFunction,

    #[error("The specified function exists, but has overloaded versions which are ambiguous given the argument types provided")]
    AmbiguousFunction,

    #[error("Can only dynamically call plain functions")]
    UnsupportedFunctionType,

    #[error("Functions with OUT/IN_OUT/TABLE arguments are not supported")]
    UnsupportedArgumentModes,

    #[error("Functions with argument or return types of `internal` are not supported")]
    InternalTypeNotSupported,

    #[error("The requested return type `{0}` is not compatible with the actual return type `{1}`")]
    IncompatibleReturnType(pg_sys::Oid, pg_sys::Oid),

    #[error("Function call has more arguments than are supported")]
    TooManyArguments,

    #[error("Did not provide enough non-default arguments")]
    NotEnoughArguments,

    #[error("Function has no default arguments")]
    NoDefaultArguments,

    #[error("Argument #{0} does not have a DEFAULT value")]
    NotDefaultArgument(usize),

    #[error("Argument's default value is not a constant expression")]
    DefaultNotConstantExpression,
}
```

## Calling a Function

The top-level function `fn_call()` is what is used to dynamically call a function.  Its signature is:

```rust
pub fn fn_call<R: FromDatum + IntoDatum>(
    fname: &str,
    args: &[&dyn FnCallArg],
) -> Result<Option<R>, FnCallError>
```

`fn_call` itself takes two arguments.  The first, `fname` is the (possibly schema-qualified) function name, as a string.


The second argument, `args`, is a slice of `FnCallArg` dyn references (these are written using `&Arg::XXX`).  And it 
returns a `Result<Option<R>, FnCallError>`.

An `Ok` response will either contain `Some(R)` if the called function returned a non-null value, or `None` if it did.

An `Err` response will contain one of the `FnCallError` variants detailed above, indicating the problem encountered 
while trying to call the function.  It is guaranteed that if `fn_call` returns an `Err`, then the desired function was
**not** called.

If the called function raises a Postgres `ERROR` then the current transaction is aborted and control is returned back
to Postgres, not the caller.  This is typical Postgres and PL/Rust behavior in the face of an `ERROR` or Rust panic. 

## Simple Example

First, lets define a SQL function that sums the elements of an `int[]`.  We're using a `LANGUAGE sql` function here
to demonstrate how PL/Rust can call functions of any other language:

```sql
CREATE OR REPLACE FUNCTION sum_array(a int[]) RETURNS int STRICT LANGUAGE sql AS $$ SELECT sum(e) FROM unnest(a) e $$;
```

Now, lets call this function from a PL/Rust function:

```sql
CREATE OR REPLACE FUNCTION transform_array(a int[]) RETURNS int STRICT LANGUAGE plrust AS $$
    let a = a.into_iter().map(|e| e.unwrap_or(0) + 1).collect::<Vec<_>>();  // add one to every element of the array
    Ok(fn_call("sum_array", &[&Arg::Value(a)])?)
$$;

SELECT transform_array(ARRAY[1,2,3]);
transform_array 
-----------------
               9
(1 row)
```

## Complex Example

This is contrived, of course, but lets make a PL/Rust function with a few different argument types and have it simply
convert their values to a debug-formatted String.  Then we'll call that function from another PL/Rust function.

```sql
CREATE OR REPLACE FUNCTION debug_format_args(a text, b bigint, c float4 DEFAULT 0.99) RETURNS text LANGUAGE plrust AS $$
    Ok(Some(format!("{:?}, {:?}, {:?}", a, b, c)))  
$$;

SELECT debug_format_args('hi', NULL);
      debug_format_args       
------------------------------
 Some("hi"), None, Some(0.99)
(1 row)
```

Now, lets call it from another PL/Rust function using these same argument values.  Which is `'hi'` for the first argument,
NULL for the second, and using the default value for the third:

```sql
CREATE OR REPLACE FUNCTION complex_example() RETURNS text LANGUAGE plrust AS $$
    let result = fn_call("debug_format_args", &[&Arg::Value("hi"), &Arg::<i64>::Null, &Arg::<f32>::Default])?;
    Ok(result)    
$$;

SELECT complex_example();
complex_example        
------------------------------
 Some("hi"), None, Some(0.99)
(1 row)
```

You'll notice here that the `Arg::Null` and `Arg::Default` argument values are typed with `::<i64>` and `::<f32>` 
respectively.  It is necessary for PL/Rust to know the types of each argument at compile time, so that during runtime
the proper function can be chosen.  This helps to ensure there's no ambiguity related to Postgres' function overloading
features.  For example, now let's overload `debug_format_args` with a different type for the second argument:

```sql
CREATE OR REPLACE FUNCTION debug_format_args(a text, b bool, c float4 DEFAULT 0.99) RETURNS text LANGUAGE plrust AS $$
    Ok(Some(format!("{:?}, {:?}, {:?}", a, b, c)))  
$$;

SELECT debug_format_args('hi', NULL);
ERROR:  42725: function debug_format_args(unknown, unknown) is not unique
LINE 1: SELECT debug_format_args('hi', NULL);
               ^
HINT:  Could not choose a best candidate function. You might need to add explicit type casts.
```

As you can see, even Postgres can't figure out which `debug_format_args` function to call as it doesn't know the intended
type of the second `NULL` argument.  We can tell it, of course:

```sql
SELECT debug_format_args('hi', NULL::bool);
      debug_format_args       
------------------------------
 Some("hi"), None, Some(0.99)
(1 row)
```

Note that if we call our `complex_example` function again, now that we've added another version of `debug_format_args`, 
it *still* calls the correct one -- the version with an `int` as the second argument.


## Limitations

PL/Rust does **not** support dynamically calling functions with `OUT` or `IN OUT` arguments.  Nor does it support 
calling functions that return `SETOF $type` or `TABLE(...)`.  

It is possible these limitations will be lifted in a future version.

