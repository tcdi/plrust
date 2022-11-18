use std::ffi::CStr;

use pgx::pg_sys::DropStmt;
use pgx::{
    pg_guard, pg_sys, FromDatum, IntoDatum, PgBox, PgBuiltInOids, PgList, PgLogLevel,
    PgSqlErrorCode, Spi,
};

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
unsafe fn plrust_process_utility_hook_internal(
    pstmt: *mut pg_sys::PlannedStmt,
    query_string: *const ::std::os::raw::c_char,
    read_only_tree: bool,
    context: pg_sys::ProcessUtilityContext,
    params: pg_sys::ParamListInfo,
    query_env: *mut pg_sys::QueryEnvironment,
    dest: *mut pg_sys::DestReceiver,
    qc: *mut pg_sys::QueryCompletion,
) {
    let pstmt = PgBox::from_pg(pstmt);
    let utility_stmt = PgBox::from_pg(pstmt.utilityStmt);

    // examine the UtilityStatement itself.  We're interested in three different commands,
    // "DROP FUNCTION", "DROP SCHEMA", and "ALTER FUNCTION".  Two different node types are used
    // for these -- `DropStmt` and `AlterFunctionStmt`

    if utility_stmt.type_ == pg_sys::NodeTag_T_DropStmt {
        let drop_stmt = PgBox::from_pg(pstmt.utilityStmt.cast::<pg_sys::DropStmt>());

        // in the case of DropStmt, if the object being dropped is a FUNCTION or a SCHEMA, we'll
        // go off and handle those two cases
        match drop_stmt.removeType {
            pg_sys::ObjectType_OBJECT_FUNCTION => handle_drop_function(&drop_stmt),
            pg_sys::ObjectType_OBJECT_SCHEMA => handle_drop_schema(&drop_stmt),
            _ => {
                // we don't do anything for the other objects
            }
        }
    } else if utility_stmt.type_ == pg_sys::NodeTag_T_AlterFunctionStmt {
        let alter_stmt = PgBox::from_pg(pstmt.utilityStmt.cast::<pg_sys::AlterFunctionStmt>());

        // and for AlterFunctioStmt, we'll just go do it.
        handle_alter_function(&alter_stmt);
    }

    if PREVIOUS_PROCESS_UTILITY_HOOK.is_some() {
        // previous hook must go last as we need the catalog entries this utility statement might
        // operate on to be valid
        let prev_hook = PREVIOUS_PROCESS_UTILITY_HOOK.as_ref().unwrap();
        prev_hook(
            pstmt.into_pg(),
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
        // otherwise if there isn't one, we are the first to hook ProcessUtility, so ask Postgres
        // to do whatever it wants to do with this statement
        pg_sys::standard_ProcessUtility(
            pstmt.into_pg(),
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

/// if the function being altered is `LANGUAGE plrust`, block any attempted change to the `STRICT`
/// property, even if to the current value
unsafe fn handle_alter_function(alter_stmt: &PgBox<pg_sys::AlterFunctionStmt>) {
    let pg_proc_oid = pg_sys::LookupFuncWithArgs(alter_stmt.objtype, alter_stmt.func, false);
    let lang_oid = lookup_func_lang(pg_proc_oid);

    if lang_oid == plrust_lang_oid() {
        // block a change to the 'STRICT' property
        let actions = PgList::<pg_sys::DefElem>::from_pg(alter_stmt.actions);
        for defelem in actions.iter_ptr() {
            static STRICT: &str = "strict";

            let defelem = PgBox::from_pg(defelem);
            let name = CStr::from_ptr(defelem.defname);

            if name.to_string_lossy().eq_ignore_ascii_case(STRICT) {
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
unsafe fn handle_drop_function(drop_stmt: &PgBox<DropStmt>) {
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
unsafe fn handle_drop_schema(drop_stmt: &PgBox<DropStmt>) {
    for object in objects(drop_stmt, pg_sys::NamespaceRelationId) {
        for pg_proc_oid in all_in_namespace(object.objectId) {
            plrust_proc::drop_function(pg_proc_oid)
        }
    }
}

/// Returns an iterator over the `objects` in the `[DropStmt]`, filtered by only those of the
/// specified `filter_class_id`
unsafe fn objects(
    drop_stmt: &PgBox<DropStmt>,
    filter_class_id: pg_sys::Oid,
) -> impl Iterator<Item = pg_sys::ObjectAddress> {
    PgList::<pg_sys::Node>::from_pg(drop_stmt.objects)
        .iter_ptr()
        .map(move |object| {
            let mut rel = std::ptr::null_mut();
            pg_sys::get_object_address(
                drop_stmt.removeType,
                object,
                &mut rel,
                pg_sys::AccessExclusiveLock as pg_sys::LOCKMODE,
                drop_stmt.missing_ok,
            )
        })
        .filter(move |oa| oa.classId == filter_class_id && oa.objectId != pg_sys::InvalidOid)
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
unsafe fn lookup_func_lang(pg_proc_oid: pg_sys::Oid) -> Option<pg_sys::Oid> {
    let cache_entry = pg_sys::SearchSysCache1(
        pg_sys::SysCacheIdentifier_PROCOID as i32,
        pg_proc_oid.into_datum().unwrap(),
    );
    if !cache_entry.is_null() {
        let mut is_null = false;
        let lang_datum = pg_sys::SysCacheGetAttr(
            pg_sys::SysCacheIdentifier_PROCOID as i32,
            cache_entry,
            pg_sys::Anum_pg_proc_prolang as pg_sys::AttrNumber,
            &mut is_null,
        );
        // SAFETY:  the datum will never be null -- postgres has a NOT NULL constraint on prolang
        let lang_oid = pg_sys::Oid::from_datum(lang_datum, is_null).unwrap();
        pg_sys::ReleaseSysCache(cache_entry);

        Some(lang_oid)
    } else {
        None
    }
}
