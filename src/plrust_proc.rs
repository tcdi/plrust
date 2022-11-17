//! Routines for managing the `plrust.plrust_proc` extension table along with the data it contains
use crate::error::PlRustError;
use crate::gucs;
use crate::user_crate::{StateLoaded, UserCrate};
use pgx::pg_sys::MyDatabaseId;
use pgx::{extension_sql, pg_sys, IntoDatum, PgBuiltInOids, PgOid, Spi};
use std::path::Path;

extension_sql!(
    r#"
CREATE TYPE supported_arch AS ENUM (
    'aarch64',
    'x86_64'
);
CREATE TABLE plrust_proc (
    id   regproc            NOT NULL,
    arch supported_arch     NOT NULL,
    os   text               NOT NULL,
    so   bytea              NOT NULL,
    PRIMARY KEY(id, arch, os)
    --
    -- Would be nice if we could make a foreign key over to pg_catalog.pg_proc
    -- but that's okay.  We'll be managing access to this table ourselves
    --
    -- CONSTRAINT ft_pg_proc_oid FOREIGN KEY(id) REFERENCES pg_catalog.pg_proc(oid)
);
SELECT pg_catalog.pg_extension_config_dump('plrust_proc', '');

"#,
    name = "extschema"
);

/// Insert a new row into the `plrust.plrust_proc` table to represent the shared library at the
/// specified `so_path`.  This function intuits the `arch` and `os` values from the current runtime
#[tracing::instrument(level = "debug")]
pub(crate) fn insert(pg_proc_oid: pg_sys::Oid, so_path: &Path) -> eyre::Result<()> {
    let so = std::fs::read(so_path)?;
    let mut args = pkey_datums(pg_proc_oid);
    args.push((PgBuiltInOids::BYTEAOID.oid(), so.into_datum()));

    Spi::run_with_args(
            "INSERT INTO plrust.plrust_proc(id, arch, os, so) VALUES ($1, $2::plrust.supported_arch, $3, $4)",
            Some(args),
        );
    Ok(())
}

/// Dynamically load the shared library stored in `plrust.plrust_proc` for the specified `pg_proc_oid`
/// procedure object id.  The function intuits the `arch` and `os` values from the current runtime
#[tracing::instrument(level = "debug")]
pub(crate) fn load(pg_proc_oid: pg_sys::Oid) -> eyre::Result<UserCrate<StateLoaded>> {
    // using SPI, read the plrust_proc entry for the provided pg_proc.oid value
    let so = Spi::get_one_with_args::<&[u8]>(
        "SELECT so FROM plrust.plrust_proc WHERE (id, arch, os) = ($1, $2::plrust.supported_arch, $3)",
        pkey_datums(pg_proc_oid),
        )
        .ok_or(PlRustError::NoProcEntry(
            pg_proc_oid,
            std::env::consts::ARCH,
            std::env::consts::OS,
        ))?;

    // we write the shared object (`so`) bytes out to a temporary file rooted in our
    // configured `plrust.work_dir`.  This will get removed from disk when this function
    // exists, which is fine because we'll have dlopen()'d it by then and no longer need it
    let work_dir = gucs::work_dir();
    let temp_so_file = tempfile::Builder::new().tempfile_in(work_dir)?;
    std::fs::write(&temp_so_file, so)?;

    // SAFETY: Postgres globally sets this to `const InvalidOid`, so is always read-safe,
    // then writes it only during initialization, so we should not be racing anyone.
    let db_oid = unsafe { MyDatabaseId };

    // fabricate a StateBuilt version of the UserCrate so that we can "load()" it -- tho we're
    // long since past the idea of crates, but whatev, I just work here
    let built = UserCrate::built(db_oid, pg_proc_oid, temp_so_file.path());
    let loaded = unsafe { built.load()? };

    // just to be clear, the temp_so_file gets deleted here -- it does not stick around after
    // we've dlopen()'d it
    drop(temp_so_file);

    // all good
    Ok(loaded)
}

fn pkey_datums(pg_proc_oid: pg_sys::Oid) -> Vec<(PgOid, Option<pg_sys::Datum>)> {
    vec![
        (PgBuiltInOids::REGPROCOID.oid(), pg_proc_oid.into_datum()),
        (
            PgBuiltInOids::TEXTOID.oid(),
            std::env::consts::ARCH.into_datum(),
        ),
        (
            PgBuiltInOids::TEXTOID.oid(),
            std::env::consts::OS.into_datum(),
        ),
    ]
}
