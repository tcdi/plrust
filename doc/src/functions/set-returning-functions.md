# Set Returning Functions

PL/Rust supports both set returning function styles, `RETURNS SETOF $type` and `RETURNS TABLE (...)`.  In both cases,
the function returns a specialized `Iterator` for the specific style.

It's useful to think of set returning functions as returning something that resembles a table, either with one unnamed column (`RETURNS SETOF`)
or multiple, named columns (`RETURNS TABLE`).

In both cases, the Iterator Item type is an `Option<T>`, where `T` is the [return type](return-type.md).  The reason
for this is that PL/Rust needs to allow a returned row/tuple to be NULL (`Option::None`).

## `RETURNS SETOF $type`

`RETURNS SETOF $type` returns a "table" with one, unnamed column.  Each returned row must be an `Option` of the return type,
either `Some(T)` or `None`, indicating NULL.

A simple example of splitting a text string on whitespace, following Rust's rules:

```sql
CREATE OR REPLACE FUNCTION split_whitespace(s text) RETURNS SETOF text STRICT LANGUAGE plrust AS $$
    let by_whitespace = s.split_whitespace();   // borrows from `s` which is a `&str`
    let mapped = by_whitespace.map(|token| {
        if token == "this" { None }     // just to demonstrate returning a NULL 
        else { Some(token.to_string()) }
    });
    let iter = SetOfIterator::new(mapped);
    Ok(Some(iter)) 
$$;
```

PL/Rust generates the following method signature for the above function: 

```rust
fn plrust_fn_oid_19691_336344<'a>(
    s: &'a str,
) -> ::std::result::Result< // the function itself can return a `Result::Err`
    Option< // if `Option::None` will return zero rows
        ::pgrx::iter::SetOfIterator< // indicates returning a set of values
            'a, // allows borrowing from `s` 
            Option<String>  // and the type is an optional, owned string
        > 
    >,
    Box<dyn std::error::Error + Send + Sync + 'static>, // boilerplate error type
> {
   //  <your code here>
}
```

And finally, its result:

```sql
SELECT * FROM split_whitespace('hello world, this is a plrust set returning function');
split_whitespace 
------------------
 hello
 world,
             -- remember we returned `None` for the token "this" 
 is
 a
 plrust
 set
 returning
 function
 (9 rows)
```

## `RETURNS TABLE (...)`

Returning a table with multiple named (and typed) columns is similar to returning a test.  Instead of `SetOfIterator`, 
PL/Rust uses `TableIterator`.  `TableIterator` is a Rust `Iterator` who's Item is a tuple where its field types match
those of the UDF being created:

```sql
CREATE OR REPLACE FUNCTION count_words(s text) RETURNS TABLE (count int, word text) STRICT LANGUAGE plrust AS $$
    use std::collections::HashMap;
    let mut buckets: HashMap<&str, i32> = Default::default();
    
    for word in s.split_whitespace() {
        buckets.entry(word).and_modify(|cnt| *cnt += 1).or_insert(1);
    }
    
    let as_tuples = buckets.into_iter().map(|(word, cnt)| {
        ( Some(cnt), Some(word.to_string()) )
    }); 
    Ok(Some(TableIterator::new(as_tuples)))
$$;
```

PL/Rust generates this function signature:

```rust
fn plrust_fn_oid_19691_336349<'a>(
   s: &'a str,
) -> ::std::result::Result::< // the function itself can return a `Result::Err`
   Option< // if `Option::None` will return zero rows
       ::pgrx::iter::TableIterator< // indicates returning a "table" of tuples
           'a,  // allows borrowing from `s`
           (
               ::pgrx::name!(count, Option < i32 >),    // the "count" column
               ::pgrx::name!(word, Option < String >),  // the "word" column
           ),
       >,
   >,
   Box<dyn std::error::Error + Send + Sync + 'static>,
> {
    // <your code here>
}
```

And the results from this function are:

```sql
# SELECT * FROM count_words('this is a test that is testing plrust''s SRF support');
 count |   word   
-------+----------
     1 | a
     1 | test
     1 | that
     2 | is
     1 | this
     1 | testing
     1 | SRF
     1 | support
     1 | plrust's
(9 rows)
```

The important thing to keep in mind when writing PL/Rust functions that `RETURNS TABLE` is that the structure being 
returned is a Rust tuple of `Option<T>`s where each field's `T` is the [return type](return-type.md) as specified in 
the `TABLE (...)` clause.