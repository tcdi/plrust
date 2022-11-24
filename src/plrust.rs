/*
Portions Copyright 2020-2021 ZomboDB, LLC.
Portions Copyright 2021-2022 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

use crate::{
    gucs, plrust_proc,
    user_crate::{StateLoaded, UserCrate},
};

use pgx::{pg_sys::FunctionCallInfo, pg_sys::MyDatabaseId, prelude::*};
use std::{cell::RefCell, collections::HashMap, process::Output};

use crate::error::PlRustError;
use crate::pgproc::PgProc;
use crate::plrust_proc::get_target_triple;
use eyre::WrapErr;

thread_local! {
    pub(crate) static LOADED_SYMBOLS: RefCell<HashMap<pg_sys::Oid, UserCrate<StateLoaded>>> = Default::default();
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
            user_crate.close().unwrap();
        }
    })
}

#[tracing::instrument(level = "debug")]
pub(crate) unsafe fn evaluate_function(
    fn_oid: pg_sys::Oid,
    fcinfo: FunctionCallInfo,
) -> eyre::Result<pg_sys::Datum> {
    LOADED_SYMBOLS.with(|loaded_symbols| {
        let mut loaded_symbols_handle = loaded_symbols.borrow_mut();

        let user_crate_loaded = if let Some(current) = loaded_symbols_handle.get_mut(&fn_oid) {
            let current_xmin = PgProc::new(fn_oid)
                .ok_or_else(|| PlRustError::NoSuchFunction(fn_oid))?
                .xmin();

            // xmin represents the transaction id that inserted this row (in this case into
            // pg_catalog.pg_proc).  So if it's changed from the last time we loaded the function
            // then we have more work to do...
            if current.xmin() != current_xmin {
                // the function, which we've previously loaded, was changed by a concurrent session.
                // This could be caused by (at least) the "OR REPLACE" bit of CREATE OR REPLACE or
                // by an ALTER FUNCTION that changed one of the attributes of the function.
                tracing::trace!(
                    "Reloading function {fn_oid} due to change from concurrent session"
                );

                // load the new function
                let new = plrust_proc::load(fn_oid)?;

                // swap out the currently loaded function for the new one
                let old = std::mem::replace(current, new);

                // make a best effort to try and close the old loaded function.  If dlclose() fails,
                // there's nothing we can do but carry on with the newly loaded version
                if let Err(e) = old.close() {
                    tracing::warn!("Failed to close the old version of function {fn_oid}.  Ignoring, and continuing with new version: {e}");
                }
            }

            current
        } else {
            // loading the function for the first time
            loaded_symbols_handle
                .entry(fn_oid)
                .or_insert(plrust_proc::load(fn_oid)?)
        };

        tracing::trace!(
            "Evaluating symbol {:?} from {}",
            user_crate_loaded.symbol_name(),
            user_crate_loaded.shared_object().display()
        );

        Ok(unsafe { user_crate_loaded.evaluate(fcinfo) })
    })
}

#[tracing::instrument(level = "debug")]
pub(crate) fn compile_function(fn_oid: pg_sys::Oid) -> eyre::Result<Output> {
    let work_dir = gucs::work_dir();
    let pg_config = gucs::pg_config();
    let target_dir = work_dir.join("target");
    // SAFETY: Postgres globally sets this to `const InvalidOid`, so is always read-safe,
    // then writes it only during initialization, so we should not be racing anyone.
    let db_oid = unsafe { MyDatabaseId };

    let generated = unsafe { UserCrate::try_from_fn_oid(db_oid, fn_oid)? };
    let provisioned = generated.provision(&work_dir)?;
    // We want to introduce validation here.
    let crate_dir = provisioned.crate_dir().to_path_buf();
    let (validated, output) = provisioned.validate(pg_config, target_dir.as_path())?;;
    let (built, output) = validated.build(pg_config, target_dir.as_path())?;
    let shared_object = built.shared_object();

    // store the shared object in our table
    plrust_proc::create_or_replace_function(fn_oid, shared_object)?;

    // cleanup after ourselves
    tracing::trace!("removing {}", shared_object.display());
    std::fs::remove_file(&shared_object).wrap_err(format!(
        "Problem deleting temporary shared object file at '{}'",
        shared_object.display()
    ))?;
    tracing::trace!("removing {}", crate_dir.display());
    std::fs::remove_dir_all(&crate_dir).wrap_err(format!(
        "Problem deleting temporary crate directory at '{}'",
        crate_dir.display()
    ))?;

    Ok(output)
}

pub(crate) fn crate_name(db_oid: pg_sys::Oid, fn_oid: pg_sys::Oid) -> String {
    // Include current_platform in the name
    // There's no guarantee that the compiled library will be
    // in the same architecture if the database was restored
    let crate_name = format!(
        "plrust_fn_oid_{}_{}_{}",
        db_oid,
        fn_oid,
        get_target_triple().replace("-", "_")
    );

    crate_name
}
