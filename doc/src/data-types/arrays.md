# Arrays

Arrays of all of supported types are mapped as `Vec<Option<T>>` where `T` is the Rust mapping for the
SQL datatype.  For example, a SQL `BIGINT[]` is mapped to `Vec<Option<i64>>`, and a `TEXT[]` is mapped to 
`Vec<Option<&str>>`.

Working with arrays can be *slightly* cumbersome as Postgres allows NULL as an individual array element.  As Rust has
no concept of "null", PL/Rust uses `Option<T>` to represent the SQL idea of "I don't have a value".

```sql
CREATE FUNCTION sum_array(a INT[]) RETURNS BIGINT STRICT LANGUAGE plrust AS $$
    let sum = a.into_iter().map(|i| i.unwrap_or_default() as i64).sum();
    Ok(Some(sum))
$$;

# SELECT sum_array(ARRAY[1,2,3]::int[]);
 sum_array 
-----------
         6
```
