# Server Programming Interface (SPI)

PL/Rust provides support for PostgreSQL's [SPI](https://www.postgresql.org/docs/current/spi.html).



`Error`

`Result`

`Spi`



## Example usage

> UNDER DEVELOPMENT - Example Pseudo-query, not functional


```sql
CREATE FUNCTION spi_srf()
    RETURNS TABLE(id BIGINT)
    LANGUAGE plrust
AS
$$
    let query = "SELECT id::BIGINT FROM generate_series(1, 3) id;";

    Spi::connect(|client| {
        let mut results = Vec::new();
        let mut tup_table = client.select(query, None, None)?;

        while let Some(row) = tup_table.next() {
            let id = row["id"].value::<i64>();

            results.push((id));
        }
        Ok(TableIterator::new(results.into_iter()))
    })
$$
;
```


