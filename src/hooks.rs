#![deny(unsafe_op_in_unsafe_fn)]
use std::ffi::CStr;

use pgx::{
    pg_guard, pg_sys, IntoDatum, PgBox, PgBuiltInOids, PgList, PgLogLevel, PgSqlErrorCode, Spi,
};

use crate::pgproc::PgProc;
use crate::{plrust_lang_oid, plrust_proc};

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
    let pstmt = unsafe {
        // SAFETY:  Postgres will have provided us with a valid PlannedStmt pointer, which it allocated
        PgBox::from_pg(pstmt)
    };
    let utility_stmt = unsafe {
        // SAFETY:  and this is the "process utility hook", so pstmt's `utilityStmt` member will also
        // be a properly-allocated Postgres pointer
        PgBox::from_pg(pstmt.utilityStmt)
    };

    // examine the UtilityStatement itself.  We're interested in three different commands,
    // "DROP FUNCTION", "DROP SCHEMA", and "ALTER FUNCTION".  Two different node types are used
    // for these -- `DropStmt` and `AlterFunctionStmt`

    // Note that we handle calling the previous hook (if there is one) differently for DROP and ALTER

    if utility_stmt.type_ == pg_sys::NodeTag_T_DropStmt {
        let drop_stmt = unsafe {
            // SAFETY:  we already determined that pstmt.utilityStmt is valid, and we just determined
            // its "node type" is a DropStmt, so the cast is clean
            PgBox::from_pg(pstmt.utilityStmt.cast::<pg_sys::DropStmt>())
        };

        // in the case of DropStmt, if the object being dropped is a FUNCTION or a SCHEMA, we'll
        // go off and handle those two cases
        match drop_stmt.removeType {
            pg_sys::ObjectType_OBJECT_FUNCTION => handle_drop_function(&drop_stmt),
            pg_sys::ObjectType_OBJECT_SCHEMA => handle_drop_schema(&drop_stmt),
            _ => {
                // we don't do anything for the other objects
            }
        }

        // call the previous hook last.  We want to call it after our work because we need the catalog
        // entries in place to find/ensure that we only affect plrust functions.
        #[rustfmt::skip]
        call_prev_hook(pstmt.into_pg(), query_string, read_only_tree, context, params, query_env, dest, qc);
    } else if utility_stmt.type_ == pg_sys::NodeTag_T_AlterFunctionStmt {
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
        handle_alter_function(&alter_stmt);
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
fn handle_alter_function(alter_stmt: &PgBox<pg_sys::AlterFunctionStmt>) {
    let pg_proc_oid = unsafe {
        // SAFETY:  specifying missing_ok=false ensures that LookupFuncWithArgs won't return for
        // something that doesn't exist.
        pg_sys::LookupFuncWithArgs(alter_stmt.objtype, alter_stmt.func, false)
    };
    let lang_oid = lookup_func_lang(pg_proc_oid);

    if lang_oid == plrust_lang_oid() {
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
}

/// drop all the `LANGUAGE plrust` functions from the set of all functions being dropped by the [`DropStmt`]
fn handle_drop_function(drop_stmt: &PgBox<pg_sys::DropStmt>) {
    let plrust_lang_oid = plrust_lang_oid();

    for pg_proc_oid in objects(drop_stmt, pg_sys::ProcedureRelationId).filter_map(|oa| {
        let lang_oid = lookup_func_lang(oa.objectId);
        // if it's a pl/rust function we can drop it
        (lang_oid == plrust_lang_oid).then(|| oa.objectId)
    }) {
        plrust_proc::drop_function(pg_proc_oid);
    }
}

/// drop all `LANGUAGE plrust` functions in any of the schemas being dropped by the [`DropStmt`]
fn handle_drop_schema(drop_stmt: &PgBox<pg_sys::DropStmt>) {
    for object in objects(drop_stmt, pg_sys::NamespaceRelationId) {
        for pg_proc_oid in all_in_namespace(object.objectId) {
            plrust_proc::drop_function(pg_proc_oid)
        }
    }
}

/// Returns an iterator over the `objects` in the `[DropStmt]`, filtered by only those of the
/// specified `filter_class_id`
fn objects(
    drop_stmt: &PgBox<pg_sys::DropStmt>,
    filter_class_id: pg_sys::Oid,
) -> impl Iterator<Item = pg_sys::ObjectAddress> {
    let list = unsafe {
        // SAFETY:  Postgres documents DropStmt.object as being a "list of names", which are in fact
        // compatible with generic pg_sys::Node types
        PgList::<pg_sys::Node>::from_pg(drop_stmt.objects)
    };

    list.iter_ptr()
        .filter_map(move |object| {
            let mut rel = std::ptr::null_mut();

            unsafe {
                // SAFETY:  "object" is a valid Node pointer from the list, and the value returned
                // from get_object_address is never null.  We don't particularly care about the
                // value of "missing_ok" as if the named object is missing the returned
                // "ObjectAddress.objectId" will be InvalidOid and we filter that out here
                let address = pg_sys::get_object_address(
                    drop_stmt.removeType,
                    object,
                    &mut rel,
                    pg_sys::AccessExclusiveLock as pg_sys::LOCKMODE,
                    drop_stmt.missing_ok,
                );

                if address.objectId == pg_sys::InvalidOid || address.classId != filter_class_id {
                    None
                } else {
                    Some(address)
                }
            }
        })
        .collect::<Vec<_>>()
        .into_iter()
}

/// Returns an Iterator of `LANGUAGE plrust` function oids (`pg_catalog.pg_proc.oid`) in a specific namespace
#[tracing::instrument(level = "debug")]
pub(crate) fn all_in_namespace(pg_namespace_oid: pg_sys::Oid) -> Vec<pg_sys::Oid> {
    Spi::connect(|client| {
        let results = client.select(
            r#"
                        SELECT oid
                        FROM pg_catalog.pg_proc
                        WHERE pronamespace = $1
                          AND prolang = (SELECT oid FROM pg_catalog.pg_language WHERE lanname = 'plrust')
                  "#,
            None,
            Some(vec![(
                PgBuiltInOids::OIDOID.oid(),
                pg_namespace_oid.into_datum(),
            )]),
        );

        let proc_oids = results
            .into_iter()
            .map(|row| {
                row.by_ordinal(1)
                    .ok()
                    .unwrap()
                    .value::<pg_sys::Oid>()
                    .unwrap()
            })
            .collect::<Vec<_>>();

        Ok(Some(proc_oids))
    }).unwrap()
}

/// Return the specified function's `prolang` value from `pg_catalog.pg_proc`
fn lookup_func_lang(pg_proc_oid: pg_sys::Oid) -> Option<pg_sys::Oid> {
    let meta = PgProc::new(pg_proc_oid)?;
    Some(meta.prolang())
}
