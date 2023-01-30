/*
Portions Copyright 2020-2021 ZomboDB, LLC.
Portions Copyright 2021-2023 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

use std::ffi::CStr;

use pgx::{pg_guard, pg_sys, PgBox, PgList, PgLogLevel, PgSqlErrorCode};
use pgx::pg_sys::Oid;

use crate::pgproc::PgProc;

static mut PREVIOUS_PROCESS_UTILITY_HOOK: pg_sys::ProcessUtility_hook_type = None;

pub(crate) fn init() {
    unsafe {
        // SAFETY:  Postgres will have set ProcessUtility_hook to a valid pointer (even if the
        // null pointer) long before this function is called
        PREVIOUS_PROCESS_UTILITY_HOOK = pg_sys::ProcessUtility_hook;
        pg_sys::ProcessUtility_hook = Some(plrust_process_utility_hook);
    }
}

#[cfg(feature = "pg13")]
#[pg_guard]
unsafe extern "C" fn plrust_process_utility_hook(
    pstmt: *mut pg_sys::PlannedStmt,
    query_string: *const ::std::os::raw::c_char,
    context: pg_sys::ProcessUtilityContext,
    params: pg_sys::ParamListInfo,
    query_env: *mut pg_sys::QueryEnvironment,
    dest: *mut pg_sys::DestReceiver,
    qc: *mut pg_sys::QueryCompletion,
) {
    plrust_process_utility_hook_internal(
        pstmt,
        query_string,
        false,
        context,
        params,
        query_env,
        dest,
        qc,
    )
}

#[cfg(not(feature = "pg13"))]
#[pg_guard]
unsafe extern "C" fn plrust_process_utility_hook(
    pstmt: *mut pg_sys::PlannedStmt,
    query_string: *const ::std::os::raw::c_char,
    read_only_tree: bool,
    context: pg_sys::ProcessUtilityContext,
    params: pg_sys::ParamListInfo,
    query_env: *mut pg_sys::QueryEnvironment,
    dest: *mut pg_sys::DestReceiver,
    qc: *mut pg_sys::QueryCompletion,
) {
    plrust_process_utility_hook_internal(
        pstmt,
        query_string,
        read_only_tree,
        context,
        params,
        query_env,
        dest,
        qc,
    )
}

#[allow(unused_variables)] // for `read_only_tree` under pg13
fn plrust_process_utility_hook_internal(
    pstmt: *mut pg_sys::PlannedStmt,
    query_string: *const ::std::os::raw::c_char,
    read_only_tree: bool,
    context: pg_sys::ProcessUtilityContext,
    params: pg_sys::ParamListInfo,
    query_env: *mut pg_sys::QueryEnvironment,
    dest: *mut pg_sys::DestReceiver,
    qc: *mut pg_sys::QueryCompletion,
) {
    let plrust_lang_oid = plrust_lang_oid();
    if plrust_lang_oid == pg_sys::Oid::INVALID {
        // it's okay if the plrust language isn't installed in this database -- we just won't do anything
        //
        // plrust must be configured as a `shared_preload_libraries` entry, so this hook will be
        // running in every database, including those without the plrust extension
        #[rustfmt::skip]
        return call_prev_hook(pstmt, query_string, read_only_tree, context, params, query_env, dest, qc);
    }

    let pstmt = unsafe {
        // SAFETY:  Postgres will have provided us with a valid PlannedStmt pointer, which it allocated
        PgBox::from_pg(pstmt)
    };
    let utility_stmt = unsafe {
        // SAFETY:  and this is the "process utility hook", so pstmt's `utilityStmt` member will also
        // be a properly-allocated Postgres pointer
        PgBox::from_pg(pstmt.utilityStmt)
    };

    // examine the UtilityStatement itself.  We're only interested in  "ALTER FUNCTION".
    if utility_stmt.type_ == pg_sys::NodeTag_T_AlterFunctionStmt {
        // for ALTER FUNCTION we call the previous hook first as it could decide it needs to change
        // the STRICT-ness of the function and we absolutely need to stop that in its tracks
        #[rustfmt::skip]
        call_prev_hook(pstmt.as_ptr(), query_string, read_only_tree, context, params, query_env, dest, qc);

        // Now we can carry on with our work
        let alter_stmt = unsafe {
            // SAFETY:  we already determined that pstmt.utilityStmt is valid, and we just determined
            // its "node type" is an AlterFunctionStmt, so the cast is clean
            PgBox::from_pg(pstmt.utilityStmt.cast::<pg_sys::AlterFunctionStmt>())
        };

        // and for AlterFunctionStmt, we'll just go do it.
        handle_alter_function(&alter_stmt, plrust_lang_oid).expect("failed to ALTER FUNCTION");
    } else {
        // this is not a utility statement we care about, so call the previous hook
        #[rustfmt::skip]
        call_prev_hook(pstmt.as_ptr(), query_string, read_only_tree, context, params, query_env, dest, qc);
    }
}

fn call_prev_hook(
    pstmt: *mut pg_sys::PlannedStmt,
    query_string: *const ::std::os::raw::c_char,
    read_only_tree: bool,
    context: pg_sys::ProcessUtilityContext,
    params: pg_sys::ParamListInfo,
    query_env: *mut pg_sys::QueryEnvironment,
    dest: *mut pg_sys::DestReceiver,
    qc: *mut pg_sys::QueryCompletion,
) {
    unsafe {
        // SAFETY:  PREVIOUS_PROCESS_UTILITY_HOOK is ours and is initialized to None, and it's
        // potentially replaced with a Some() value in `init()`.  Additionally, Postgres and plrust
        // are not threaded, so there's no chance of concurrently modifying this thing
        if PREVIOUS_PROCESS_UTILITY_HOOK.is_some() {
            let prev_hook = PREVIOUS_PROCESS_UTILITY_HOOK.as_ref().unwrap();
            prev_hook(
                pstmt,
                query_string,
                #[cfg(not(feature = "pg13"))]
                read_only_tree,
                context,
                params,
                query_env,
                dest,
                qc,
            );
        } else {
            // we are the first to hook ProcessUtility.  Tell Postgres to do whatever it wants to do
            // with this statement
            pg_sys::standard_ProcessUtility(
                pstmt,
                query_string,
                #[cfg(not(feature = "pg13"))]
                read_only_tree,
                context,
                params,
                query_env,
                dest,
                qc,
            );
        }
    }
}

/// if the function being altered is `LANGUAGE plrust`, block any attempted change to the `STRICT`
/// property, even if to the current value
fn handle_alter_function(
    alter_stmt: &PgBox<pg_sys::AlterFunctionStmt>,
    plrust_lang_oid: pg_sys::Oid,
) -> eyre::Result<()> {
    let pg_proc_oid = unsafe {
        // SAFETY:  specifying missing_ok=false ensures that LookupFuncWithArgs won't return for
        // something that doesn't exist.
        pg_sys::LookupFuncWithArgs(alter_stmt.objtype, alter_stmt.func, false)
    };
    let lang_oid = lookup_func_lang(pg_proc_oid)?;

    if lang_oid == plrust_lang_oid {
        // block a change to the 'STRICT' property
        let actions = unsafe {
            // SAFETY:  AlterFunctionStmt.actions is known to be a "list of DefElem"
            PgList::<pg_sys::DefElem>::from_pg(alter_stmt.actions)
        };
        for defelem in actions.iter_ptr() {
            let defelem = unsafe {
                // SAFETY: a Postgres pg_sys::List, and by extension PgList, wont contain null
                // pointers, so we know this DefElem pointer is valid
                PgBox::from_pg(defelem)
            };
            let name = unsafe {
                // SAFETY:  DefElem.defname is always a valid pointer to a null-terminted C string
                CStr::from_ptr(defelem.defname)
            };

            // if the defelem name contains "strict", we need to stop the show.  Changing the
            // "strict-ness" of a pl/rust function without also changing the **source code** of that
            // function and re-compiling it would cause all sorts of undefined and unexpected
            // behavior.  Declaring a function as "STRICT" means plrust/pgx doesn't need to treat
            // the arguments as `Option<T>` and instead as simply `T`.  These things are not compatible!
            //
            // We could go through the trouble of checking the current strict-ness of the function
            // and if they're the same, just carry on, but lets not complicate the code/logic for
            // something that's pretty unlikely to be a common occurrence.
            if name
                .to_string_lossy()
                .to_ascii_lowercase()
                .contains("strict")
            {
                pgx::ereport!(PgLogLevel::ERROR,
                    PgSqlErrorCode::ERRCODE_FEATURE_NOT_SUPPORTED,
                    "plrust functions cannot have their STRICT property altered",
                    "Use 'CREATE OR REPLACE FUNCTION' to alter the STRICT-ness of an existing plrust function"
                );
            }
        }
    }
    Ok(())
}

/// Return the specified function's `prolang` value from `pg_catalog.pg_proc`
fn lookup_func_lang(pg_proc_oid: pg_sys::Oid) -> eyre::Result<pg_sys::Oid> {
    let meta = PgProc::new(pg_proc_oid)?;
    Ok(meta.prolang())
}

/// Returns [`pg_sys::Oid::INVALID`] if the `plrust` language isn't installed in the current database
fn plrust_lang_oid() -> pg_sys::Oid {
    static PLRUST_LANG_NAME: &[u8] = b"plrust\0"; // want this to look like a c string

    unsafe {
        // SAFETY:  FFI is always unsafe
        //
        // If for some reason we're not currently in a transaction, we can't lookup the plrust
        // language Oid, so we must return the best value we can: Oid::INVALID
        if !pg_sys::IsTransactionState() {
            return Oid::INVALID
        }
    }

    // SAFETY: We pass `missing_ok: true`, which will return `Oid::INVALID` if the plrust language
    // isn't installed in the current database. The first parameter has the same requirements as `&CStr`.
    unsafe { pg_sys::get_language_oid(PLRUST_LANG_NAME.as_ptr().cast(), true) }
}
