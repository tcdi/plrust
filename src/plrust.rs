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
use std::{
    cell::RefCell,
    collections::{hash_map::Entry, HashMap},
    process::Output,
};

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
            tracing::info!("unloaded function");
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
        let user_crate_loaded = match loaded_symbols_handle.entry(fn_oid) {
            entry @ Entry::Occupied(_) => {
                entry.or_insert_with(|| unreachable!("Occupied entry was vacant"))
            }
            entry @ Entry::Vacant(_) => {
                let loaded = plrust_proc::load(fn_oid)?;
                entry.or_insert(loaded)
            }
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
    let crate_dir = provisioned.crate_dir().to_path_buf();
    let (built, output) = provisioned.build(pg_config, target_dir.as_path())?;
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
