# Arrays

Arrays of all of supported types are mapped as `Array<T>` where `T` is the Rust mapping for the
SQL datatype.  For example, a SQL `BIGINT[]` is mapped to `Array<i64>`, and a `TEXT[]` is mapped to
`Array<&str>`.

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

## Iteration and Slices

Pl/Rust Arrays support slices over the backing Array data if it's an array of a primitive type (i8/16/32/64, f32/64).
This can provide drastic performance improvements and even help lead to the Rust compiler autovectorizing code.

Let's examine this using arrays of random `FLOAT4` values:

```sql
CREATE OR REPLACE FUNCTION random_floats(many int) RETURNS float4[] STRICT PARALLEL SAFE LANGUAGE sql AS $$
    SELECT array_agg(random()) FROM generate_series(1, many)
$$;

CREATE TABLE floats AS SELECT random_floats(1000) f FROM generate_series(1, 100000);
```

Next, we'll sum the array using a function similar to the above:

```sql
CREATE OR REPLACE FUNCTION sum_array(a float4[]) RETURNS float4 STRICT LANGUAGE plrust AS $$
    let sum = a.into_iter().map(|i| i.unwrap_or_default()).sum();
    Ok(Some(sum))
$$;

# explain analyze select sum_array(f) from floats;
QUERY PLAN                                                   
---------------------------------------------------------------------------------------------------------------
 Seq Scan on floats  (cost=0.00..23161.32 rows=86632 width=4) (actual time=0.064..981.105 rows=100000 loops=1)
 Planning Time: 0.037 ms
 Execution Time: 983.753 ms
```

Since in this case we know that the input array won't contain null values, we can optimize slightly.  This does a fast
"O(1)" check for NULLs when creating the iterator, rather than checking each individual element during iteration:

```sql
CREATE OR REPLACE FUNCTION sum_array_no_nulls(a float4[]) RETURNS float4 STRICT LANGUAGE plrust AS $$
    let sum = a.iter_deny_null().sum();
    Ok(Some(sum))
$$;

explain analyze select sum_array_no_nulls(f) from floats;
QUERY PLAN                                                   
----------------------------------------------------------------------------------------------------------------
 Seq Scan on floats  (cost=0.00..26637.00 rows=100000 width=4) (actual time=0.055..672.365 rows=100000 loops=1)
 Planning Time: 0.035 ms
 Execution Time: 676.243 ms
```

Next, lets take a look at converting the input array into a slice before summing the values.  This is particularly fast
as it's a true "zero copy" operation:

```sql
CREATE OR REPLACE FUNCTION sum_array_slice(a float4[]) RETURNS float4 STRICT LANGUAGE plrust AS $$
    let slice = a.as_slice()?;  // use the `?` operator as not all `Array<T>`s can be converted into a slice
    let sum = slice.iter().sum();
    Ok(Some(sum))
$$;

explain analyze select sum_array_slice(f) from floats;
QUERY PLAN                                                   
----------------------------------------------------------------------------------------------------------------
 Seq Scan on floats  (cost=0.00..26637.00 rows=100000 width=4) (actual time=0.055..478.635 rows=100000 loops=1)
 Planning Time: 0.036 ms
 Execution Time: 482.344 ms
```

Finally, lets do some magic to coax the Rust compiler into autovectorizing our "sum_array" function.  The code for this
comes from, interestingly enough, Stack Overflow:  https://stackoverflow.com/questions/23100534/how-to-sum-the-values-in-an-array-slice-or-vec-in-rust/67191480#67191480

```sql
CREATE OR REPLACE FUNCTION sum_array_simd(a float4[]) RETURNS float4 STRICT LANGUAGE plrust AS $$
    use std::convert::TryInto;
    
    const LANES: usize = 16;
    
    pub fn simd_sum(values: &[f32]) -> f32 {
        let chunks = values.chunks_exact(LANES);
        let remainder = chunks.remainder();
    
        let sum = chunks.fold([0.0f32; LANES], |mut acc, chunk| {
            let chunk: [f32; LANES] = chunk.try_into().unwrap();
            for i in 0..LANES {
                acc[i] += chunk[i];
            }
            acc
        });
    
        let remainder: f32 = remainder.iter().copied().sum();
    
        let mut reduced = 0.0f32;
        for i in 0..LANES {
            reduced += sum[i];
        }
        reduced + remainder
    }

    let slice = a.as_slice()?;
    let sum = simd_sum(slice);
    Ok(Some(sum))
$$;


explain analyze select sum_array_simd(f) from floats;
QUERY PLAN                                                   
----------------------------------------------------------------------------------------------------------------
 Seq Scan on floats  (cost=0.00..26637.00 rows=100000 width=4) (actual time=0.054..413.702 rows=100000 loops=1)
 Planning Time: 0.038 ms
 Execution Time: 417.237 ms
```
