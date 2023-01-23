/*
Copyright 2021-2023 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

//! Routines for managing the `pg_catalog.pg_proc.prosrc` entry for plrust functions
use std::collections::BTreeMap;
use std::rc::Rc;

use pgx::pg_sys;
use pgx::pg_sys::MyDatabaseId;
use pgx::prelude::PgHeapTuple;
use serde::{Deserialize, Serialize};

use crate::error::PlRustError;
use crate::pgproc::PgProc;
use crate::target;
use crate::target::CompilationTarget;
use crate::user_crate::{FnReady, UserCrate};

#[derive(Default, Debug, Serialize, Deserialize)]
struct ProSrcEntry {
    src: String,
    lib: BTreeMap<String, Vec<u8>>,
}

impl TryFrom<&PgProc> for ProSrcEntry {
    type Error = serde_json::Error;

    fn try_from(pg_proc: &PgProc) -> Result<Self, Self::Error> {
        serde_json::from_str::<ProSrcEntry>(&pg_proc.prosrc())
    }
}

impl Into<String> for ProSrcEntry {
    fn into(self) -> String {
        serde_json::to_string(&self).expect("unable to serialize ProSrcEntry to json")
    }
}

impl ProSrcEntry {
    fn take_so_bytes(
        &mut self,
        compilation_target: &CompilationTarget,
    ) -> Result<Vec<u8>, PlRustError> {
        self.lib
            .remove(compilation_target.as_str())
            .ok_or_else(|| PlRustError::FunctionNotCompiledForTarget(compilation_target.clone()))
    }
}

/// Update the entry for the specified function in `pg_catalog.pg_proc.prosrc` to include the compiled
/// `so_bytes` mapped to the specified `target_triple`
#[tracing::instrument(level = "debug")]
pub(crate) fn create_or_replace_function(
    pg_proc_oid: pg_sys::Oid,
    target_triple: CompilationTarget,
    so_bytes: Vec<u8>,
) -> eyre::Result<()> {
    let pg_proc = PgProc::new(pg_proc_oid)?;
    let mut entry = ProSrcEntry::try_from(&pg_proc).unwrap_or_else(|_| {
        // the pg_proc.prosrc didn't parse as json, so assume it's just the raw function source code
        // likely means it's the first time this function is being CREATEd
        let mut entry = ProSrcEntry::default();
        entry.src = pg_proc.prosrc();
        entry
    });

    // always replace any existing bytes for the specified target_triple.  we only trust
    // what was given to us
    entry.lib.insert(target_triple.to_string(), so_bytes);

    let mut ctid = pg_proc.ctid();
    let relation = PgProc::relation();
    let tupdesc = relation.tuple_desc();
    let mut heap_tuple = unsafe {
        // SAFETY:  The `tupdesc` is based on the "pg_catalog.pg_proc" system catalog table which
        // exactly matches the `pg_proc.heap_tuple()`, which is ultimately provided by
        // a "SysCache" entry from that same catalog table.
        PgHeapTuple::from_heap_tuple(tupdesc, pg_proc.heap_tuple())
    }
    .into_owned();
    let prosrc_value: String = entry.into();
    heap_tuple.set_by_name("prosrc", prosrc_value)?;

    // TODO:  [`pgx::PgHeapTuple`] really needs a `.into_pg() -> *mut pg_sys::HeapTupleData` function.
    //        in the meantime, `.into_trigger_datum()` essentially does what that function should do,
    //        we just need to cast it to the right pointer type
    let datum = heap_tuple.into_trigger_datum().unwrap();
    let heap_tuple = datum.cast_mut_ptr();

    unsafe {
        // SAFETY:  We know that `relation` is valid because we made it above, the `ctid` represents
        // a valid row on disk because the SysCache gave it to us, and `heap_tuple` is a valid pointer
        // to a `pg_sys::HeapTupleData` because `.into_trigger_datm()` gave that to us above
        pg_sys::CatalogTupleUpdate(relation.as_ptr(), &mut ctid, heap_tuple);
    }
    Ok(())
}

/// Dynamically load the shared library stored in `pg_catalog.pg_proc.prosrc` for the specified `pg_proc_oid`
/// procedure object id and the `target_triple` of the host.
#[tracing::instrument(level = "debug")]
pub(crate) fn load(pg_proc_oid: pg_sys::Oid) -> eyre::Result<Rc<UserCrate<FnReady>>> {
    tracing::debug!("loading function oid `{pg_proc_oid}`");
    let pg_proc = PgProc::new(pg_proc_oid)?;
    let mut entry = ProSrcEntry::try_from(&pg_proc)?;
    let this_target = target::tuple()?;
    let so_bytes = entry.take_so_bytes(this_target)?;

    // SAFETY: Postgres globally sets this to `const InvalidOid`, so is always read-safe,
    // then writes it only during initialization, so we should not be racing anyone.
    let db_oid = unsafe { MyDatabaseId };

    // fabricate a FnLoad version of the UserCrate so that we can "load()" it -- tho we're
    // long since past the idea of crates, but whatev, I just work here
    let built = UserCrate::built(
        pg_proc.xmin(),
        db_oid,
        pg_proc_oid,
        this_target.clone(),
        so_bytes,
    );
    let loaded = unsafe { built.load()? };

    // all good
    Ok(Rc::new(loaded))
}
