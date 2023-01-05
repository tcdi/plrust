//! Routines for managing the `plrust.plrust_proc` extension table along with the data it contains
use crate::error::PlRustError;
use crate::gucs;
use crate::pgproc::PgProc;
use crate::user_crate::{FnReady, UserCrate};
use pgx::pg_sys::MyDatabaseId;
use pgx::{extension_sql, pg_sys, spi, IntoDatum, PgBuiltInOids, PgOid, Spi};
use std::path::Path;

extension_sql!(
    r#"
CREATE TABLE plrust.plrust_proc (
    id            regproc   NOT NULL,
    target_triple text      NOT NULL,
    so            bytea     NOT NULL,
    PRIMARY KEY(id, target_triple)

    --
    -- Would be nice if we could make a foreign key over to pg_catalog.pg_proc
    -- but that's okay.  We'll be managing access to this table ourselves
    --
    -- CONSTRAINT ft_pg_proc_oid FOREIGN KEY(id) REFERENCES pg_catalog.pg_proc(oid)
);
SELECT pg_catalog.pg_extension_config_dump('plrust.plrust_proc', '');
"#,
    name = "plrust_proc"
);

/// Insert a new row into the `plrust.plrust_proc` table using the bytes of the shared library at
/// the specified `so_path`.
#[tracing::instrument(level = "debug")]
pub(crate) fn create_or_replace_function(
    pg_proc_oid: pg_sys::Oid,
    so_path: &Path,
) -> eyre::Result<()> {
    let so = std::fs::read(so_path)?;
    let mut args = pkey_datums(pg_proc_oid);
    args.push((PgBuiltInOids::BYTEAOID.oid(), so.into_datum()));

    tracing::debug!("inserting function oid `{pg_proc_oid}`");
    Ok(Spi::run_with_args(
        r#"
                INSERT INTO plrust.plrust_proc(id, target_triple, so)
                     VALUES ($1, $2, $3)
                     ON CONFLICT (id, target_triple)
                        DO UPDATE SET so = $3
                "#,
        Some(args),
    )?)
}

#[tracing::instrument(level = "debug")]
pub(crate) fn drop_function(pg_proc_oid: pg_sys::Oid) -> spi::Result<()> {
    tracing::debug!("deleting function oid `{pg_proc_oid}`");
    Spi::run_with_args(
        "DELETE FROM plrust.plrust_proc WHERE id = $1",
        Some(vec![(
            PgBuiltInOids::REGPROCOID.oid(),
            pg_proc_oid.into_datum(),
        )]),
    )
}

/// Dynamically load the shared library stored in `plrust.plrust_proc` for the specified `pg_proc_oid`
/// procedure object id and the `target_triple` of the host.
#[tracing::instrument(level = "debug")]
pub(crate) fn load(pg_proc_oid: pg_sys::Oid) -> eyre::Result<UserCrate<FnReady>> {
    tracing::debug!("loading function oid `{pg_proc_oid}`");
    // using SPI, read the plrust_proc entry for the provided pg_proc.oid value
    let so = Spi::get_one_with_args::<&[u8]>(
        "SELECT so FROM plrust.plrust_proc WHERE (id, target_triple) = ($1, $2)",
        pkey_datums(pg_proc_oid),
    )?
    .ok_or_else(|| PlRustError::NoProcEntry(pg_proc_oid, get_target_triple().to_string()))?;

    // we write the shared object (`so`) bytes out to a temporary file rooted in our
    // configured `plrust.work_dir`.  This will get removed from disk when this function
    // exists, which is fine because we'll have dlopen()'d it by then and no longer need it
    let work_dir = gucs::work_dir();
    let temp_so_file = tempfile::Builder::new().tempfile_in(work_dir)?;
    std::fs::write(&temp_so_file, so)?;

    // SAFETY: Postgres globally sets this to `const InvalidOid`, so is always read-safe,
    // then writes it only during initialization, so we should not be racing anyone.
    let db_oid = unsafe { MyDatabaseId };

    // fabricate a FnLoad version of the UserCrate so that we can "load()" it -- tho we're
    // long since past the idea of crates, but whatev, I just work here
    let meta = PgProc::new(pg_proc_oid).ok_or(PlRustError::NullProcTuple)?;
    let built = UserCrate::built(
        meta.xmin(),
        db_oid,
        pg_proc_oid,
        temp_so_file.path().to_path_buf(),
    );
    let loaded = unsafe { built.load()? };

    // just to be obvious, the temp_so_file gets deleted here.  Now that it's been loaded, we don't
    // need it.  If any of the above failed and returned an Error, it'll still get deleted when
    // the function returns.
    drop(temp_so_file);

    // all good
    Ok(loaded)
}

/// helper function to build a vec of Spi arguments to be used as the composite primary key
/// `plrust.plrust_proc` needs to locate a function
#[inline]
fn pkey_datums(pg_proc_oid: pg_sys::Oid) -> Vec<(PgOid, Option<pg_sys::Datum>)> {
    vec![
        (PgBuiltInOids::REGPROCOID.oid(), pg_proc_oid.into_datum()),
        (
            PgBuiltInOids::TEXTOID.oid(),
            get_target_triple().into_datum(),
        ),
    ]
}

/// Assumes the `target_triple` for the current host is that of the one which compiled the plrust
/// extension shared library itself.
#[inline]
pub(crate) const fn get_target_triple() -> &'static str {
    // NB: This gets set in our `build.rs`
    env!("TARGET")
}
