# `plrust` Extension for PostgreSQL

## Using wasm

This is a fork of pl/rust as proof of concept to use wasm as oppose to shared object. Functions will be compiled into wasm and executed in wasmtime.

```sh
cargo pgx install
```

## spi poc
```sql
$ psql
psql (14.2)
Type "help" for help.

postgres=# DROP EXTENSION IF EXISTS plrust CASCADE;
NOTICE:  extension "plrust" does not exist, skipping
DROP EXTENSION
postgres=# CREATE EXTENSION IF NOT EXISTS plrust;
CREATE EXTENSION
postgres=# CREATE OR REPLACE FUNCTION spi_poc() RETURNS INTEGER
postgres-#     IMMUTABLE STRICT
postgres-#     LANGUAGE PLRUST AS
postgres-# $$
postgres$# [dependencies]
postgres$# [code]
postgres$#     spi::spi_exec_select_num(100)
postgres$# $$;
CREATE FUNCTION
postgres=# \df
                        List of functions
 Schema |  Name   | Result data type | Argument data types | Type 
--------+---------+------------------+---------------------+------
 public | spi_poc | integer          |                     | func
(1 row)

postgres=# SELECT spi_poc();
 spi_poc 
---------
     100
(1 row)

postgres=# 
```