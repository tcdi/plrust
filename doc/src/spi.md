# Server Programming Interface (SPI)

PL/Rust provides support for PostgreSQL's [SPI](https://www.postgresql.org/docs/current/spi.html).



`Error`

`Result`

`Spi`



## Example usage

The following function uses `SPI` to create a PostgreSQL
[Set Returning Function](https://www.postgresql.org/docs/current/functions-srf.html) (SRF).



```sql
CREATE FUNCTION spi_srf()
    RETURNS SETOF BIGINT
    LANGUAGE plrust
AS
$$
    let query = "SELECT id::BIGINT FROM generate_series(1, 3) id;";

    Spi::connect(|client| {
        let mut results = Vec::new();
        let mut tup_table = client.select(query, None, None)?;

        while let Some(row) = tup_table.next() {
            let id = row["id"].value::<i64>()?;
            results.push(id);
        }
        Ok(Some(SetOfIterator::new(results)))
    })

$$;
```

## Complex return types

PL/Rust currently [does not support `RETURNS TABLE`](https://github.com/tcdi/plrust/issues/36) or
[complex types with `RETURNS SETOF`](https://github.com/tcdi/plrust/issues/200#issuecomment-1426880622).


