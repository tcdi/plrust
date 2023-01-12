//! Routines for managing the `plrust.plrust_proc` extension table along with the data it contains
use std::ffi::CStr;
use std::rc::Rc;

use pgx::pg_sys::MyDatabaseId;
use pgx::{extension_sql, pg_sys, spi, IntoDatum, PgBuiltInOids, PgOid, Spi};

use crate::error::PlRustError;
use crate::gucs::CompilationTarget;
use crate::pgproc::PgProc;
use crate::user_crate::{FnReady, UserCrate};

extension_sql!(
    r#"
CREATE TABLE plrust.plrust_proc (
    --
    -- `id` is the "identity" column from `pg_catalog.pg_identify_object()`
    id            text      NOT NULL,
    target_triple text      NOT NULL,
    so            bytea     NOT NULL,
    PRIMARY KEY(id, target_triple)
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
    target_triple: CompilationTarget,
    so: Vec<u8>,
) -> eyre::Result<()> {
    let mut args = pkey_datums(pg_proc_oid, &target_triple);
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
        Some(vec![get_fn_identity_datum(pg_proc_oid)]),
    )
}

/// Dynamically load the shared library stored in `plrust.plrust_proc` for the specified `pg_proc_oid`
/// procedure object id and the `target_triple` of the host.
#[tracing::instrument(level = "debug")]
pub(crate) fn load(pg_proc_oid: pg_sys::Oid) -> eyre::Result<Rc<UserCrate<FnReady>>> {
    tracing::debug!("loading function oid `{pg_proc_oid}`");
    let this_target = get_target_triple();
    // using SPI, read the plrust_proc entry for the provided pg_proc.oid value
    let so_bytes = Spi::get_one_with_args::<Vec<u8>>(
        "SELECT so FROM plrust.plrust_proc WHERE (id, target_triple) = ($1, $2)",
        pkey_datums(pg_proc_oid, &this_target),
    )?
    .ok_or_else(|| PlRustError::NoProcEntry(pg_proc_oid, get_target_triple().clone()))?;

    // SAFETY: Postgres globally sets this to `const InvalidOid`, so is always read-safe,
    // then writes it only during initialization, so we should not be racing anyone.
    let db_oid = unsafe { MyDatabaseId };

    // fabricate a FnLoad version of the UserCrate so that we can "load()" it -- tho we're
    // long since past the idea of crates, but whatev, I just work here
    let meta = PgProc::new(pg_proc_oid).ok_or(PlRustError::NullProcTuple)?;
    let built = UserCrate::built(meta.xmin(), db_oid, pg_proc_oid, this_target, so_bytes);
    let loaded = unsafe { built.load()? };

    // all good
    Ok(Rc::new(loaded))
}

// helper function to build the primary key values used to query `plrust.plrust_proc` via Spi
#[rustfmt::skip]
#[inline]
fn pkey_datums(pg_proc_oid: pg_sys::Oid, target_triple: &CompilationTarget) -> Vec<(PgOid, Option<pg_sys::Datum>)> {
    vec![
        get_fn_identity_datum(pg_proc_oid),
        (PgBuiltInOids::TEXTOID.oid(), target_triple.as_str().into_datum()),
    ]
}

// helper function to build the function identity (oid, value) datum
fn get_fn_identity_datum(pg_proc_oid: pg_sys::Oid) -> (PgOid, Option<pg_sys::Datum>) {
    let oa = pg_sys::ObjectAddress {
        classId: pg_sys::ProcedureRelationId, // the "oid" of Postgres' `pg_catalog.pg_proc` table
        objectId: pg_proc_oid,
        objectSubId: 0,
    };
    let identity_ptr = unsafe {
        #[cfg(feature = "pg13")]
        {
            // SAFETY:  getObjectIdentity will raise an ERROR if the ObjectAddress we created doesn't
            // exist, otherwise it returns a properly palloc'd pointer
            pg_sys::getObjectIdentity(&oa as *const _)
        }

        #[cfg(not(feature = "pg13"))]
        {
            // SAFETY:  by setting "missing_ok" to false, getObjectIdentity will raise an ERROR if the
            // ObjectAddress we created doesn't exist, otherwise it returns a properly palloc'd pointer
            pg_sys::getObjectIdentity(&oa as *const _, false)
        }
    };
    let identity_str = unsafe {
        // SAFETY:  Postgres has given us a valid, albeit palloc'd, cstring as the result of getObjectIdentity
        CStr::from_ptr(identity_ptr).to_str().unwrap_or_else(|_| {
            pgx::error!("function {pg_proc_oid}'s identity is not a valid UTF8 string")
        })
    };

    let result = (PgBuiltInOids::TEXTOID.oid(), identity_str.into_datum());

    unsafe {
        // SAFETY: identity_ptr was previously proven valid and
        // identity_str was reallocated elsewhere in Postgres
        pg_sys::pfree(identity_ptr.cast());
    }

    result
}

/// Assumes the `target_triple` for the current host is that of the one which compiled the plrust
/// extension shared library itself.
#[inline]
pub(crate) fn get_target_triple() -> CompilationTarget {
    // NB: This gets set in our `build.rs`
    CompilationTarget::from(env!("TARGET"))
}
