/*
Portions Copyright 2020-2021 ZomboDB, LLC.
Portions Copyright 2021-2025 Technology Concepts & Design, Inc.

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE file.
*/

use std::rc::Rc;
use std::{cell::RefCell, collections::HashMap, process::Output};

use eyre::WrapErr;
use pgrx::{pg_sys::FunctionCallInfo, pg_sys::MyDatabaseId, prelude::*};

use crate::pgproc::PgProc;
use crate::{
    gucs, prosrc,
    user_crate::{FnReady, UserCrate},
};

thread_local! {
    pub(crate) static LOADED_SYMBOLS: RefCell<HashMap<pg_sys::Oid, Rc<UserCrate<FnReady>>>> = Default::default();
}

pub(crate) fn init() {
    ()
}

#[tracing::instrument(level = "debug")]
pub(crate) unsafe fn unload_function(fn_oid: pg_sys::Oid) {
    LOADED_SYMBOLS.with(|loaded_symbols| {
        let mut loaded_symbols_handle = loaded_symbols.borrow_mut();
        let removed = loaded_symbols_handle.remove(&fn_oid);
        if let Some(user_crate) = removed {
            if let Ok(user_crate) = Rc::try_unwrap(user_crate) {
                user_crate.close().unwrap();
            }
        }
    })
}

#[tracing::instrument(level = "debug")]
pub(crate) unsafe fn evaluate_function(
    fn_oid: pg_sys::Oid,
    fcinfo: FunctionCallInfo,
) -> eyre::Result<pg_sys::Datum> {
    let user_crate_loaded = LOADED_SYMBOLS.with(|loaded_symbols| {
        let mut loaded_symbols_handle = loaded_symbols.borrow_mut();

        let user_crate_loaded = if let Some(current) = loaded_symbols_handle.get_mut(&fn_oid) {
            let current_generation_number = PgProc::new(fn_oid)?.generation_number();

            // `generation_number`` represents the transaction id and command id that inserted this
            // row (in this case into pg_catalog.pg_proc).  So if it's changed from the last time we
            // loaded the function then we have more work to do...
            if current.generation_number() != current_generation_number {
                // the function, which we've previously loaded, was changed by a concurrent session.
                // This could be caused by (at least) the "OR REPLACE" bit of CREATE OR REPLACE or
                // by an ALTER FUNCTION that changed one of the attributes of the function.
                tracing::trace!(
                    "Reloading function {fn_oid} due to change from concurrent session"
                );

                // load the new function
                let new = prosrc::load(fn_oid)?;

                // swap out the currently loaded function for the new one
                let old = std::mem::replace(current, new);

                // make a best effort to try and close the old loaded function.  If dlclose() fails,
                // there's nothing we can do but carry on with the newly loaded version
                if let Ok(old) = Rc::try_unwrap(old) {
                    if let Err(e) = old.close() {
                        tracing::warn!("Failed to close the old version of function {fn_oid}.  Ignoring, and continuing with new version: {e}");
                    }
                }
            }

            current
        } else {
            // loading the function for the first time
            loaded_symbols_handle
                .entry(fn_oid)
                .or_insert(prosrc::load(fn_oid)?)
        };

        Ok::<_, eyre::Error>(user_crate_loaded.clone())
    })?;

    tracing::trace!(
        "Evaluating symbol {:?} for function {}",
        user_crate_loaded.symbol_name(),
        fn_oid
    );

    Ok(unsafe { user_crate_loaded.evaluate(fcinfo) })
}

#[tracing::instrument(level = "debug")]
pub(crate) fn compile_function(fn_oid: pg_sys::Oid) -> eyre::Result<Output> {
    let work_dir = gucs::work_dir();
    let target_dir = work_dir.join("target");
    // SAFETY: Postgres globally sets this to `const InvalidOid`, so is always read-safe,
    // then writes it only during initialization, so we should not be racing anyone.
    let db_oid = unsafe { MyDatabaseId };

    let generated = unsafe { UserCrate::try_from_fn_oid(db_oid, fn_oid)? };
    let provisioned = generated.provision(&work_dir)?;
    // We want to introduce validation here.
    let crate_dir = provisioned.crate_dir().to_path_buf();
    let (validated, _output) = provisioned.validate(target_dir.as_path())?;
    let target_builds = validated.build(target_dir.as_path())?;

    // we gotta have at least one built crate and it's for this host's target triple
    assert!(target_builds.len() >= 1);

    let mut this_output = None;
    for (built, output) in target_builds {
        if this_output.is_none() {
            this_output = Some(output)
        }
        let (target_triple, shared_object, lints) = built.into_inner();

        // store the shared objects in our table
        prosrc::create_or_replace_function(db_oid, fn_oid, target_triple, shared_object, lints)?;
    }

    // cleanup after ourselves
    tracing::trace!("removing {}", crate_dir.display());
    std::fs::remove_dir_all(&crate_dir).wrap_err(format!(
        "Problem deleting temporary crate directory at '{}'",
        crate_dir.display()
    ))?;

    Ok(this_output.unwrap())
}

/// Represents the generated name PL/Rust gives to the user's function (at least the one to which
/// we apply a `#[pg_extern]` annotation).  When the user function shared library is loaded, this
/// is the only symbol we access from the library.
///
/// The name itself doesn't matter, but we keep it closely tied to the Postgres `pg_proc` catalog
/// entry by including the that Oid value along with the database's Oid value.
pub(crate) fn symbol_name(db_oid: pg_sys::Oid, fn_oid: pg_sys::Oid) -> String {
    format!("plrust_fn_oid_{}_{}", db_oid.as_u32(), fn_oid.as_u32())
}

/// Represents the name PL/Rust gives the crate we generate to hold the user's function.  For any
/// given function we want this to be unique every time we generate a new crate for the function.
/// This is coax platforms like MacOS into properly dlopen-ing the final .so.
///
/// The value here is based on the [`symbol_name`] name plus the specified `generation_number` for
/// uniqueness.  It's up to the caller to make a generation number that is sufficiently unique
/// for the specified (`db_oid`, `fn_oid`) pair.
pub(crate) fn crate_name(
    db_oid: pg_sys::Oid,
    fn_oid: pg_sys::Oid,
    generation_number: u64,
) -> String {
    // NB:  This once included the compiling host's target triple as part of the crate name for
    // reasons about restoring a database to the same platform.
    //
    // This isn't necessary as we store the .so binaries by target triple in `pg_catalog.pg_proc.prosrc`,
    // so if we are restored to a different platform then the .so binary for this platform won't be used.
    //
    // This also drastically un-complicates what we'd otherwise have to do when cross-compiling for
    // multiple targets.
    format!("{}_{}", symbol_name(db_oid, fn_oid), generation_number)
}
