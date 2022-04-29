/*
Portions Copyright 2020-2021 ZomboDB, LLC.
Portions Copyright 2021-2022 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

mod error;
mod gucs;
mod guest_with_oids;
pub mod interface;
mod logging;
mod plrust;
mod plrust_store;
mod wasm_executor;
mod tests;

use error::PlRustError;
use pgx::*;
#[cfg(test)]
pub(crate) use tests::pg_test;

wit_bindgen_wasmtime::export!("../components/wit/host.wit");
wit_bindgen_wasmtime::import!("../components/wit/guest.wit");

pg_module_magic!();

#[pg_guard]
fn _PG_init() {
    color_eyre::config::HookBuilder::default()
        .theme(if !atty::is(atty::Stream::Stderr) {
            color_eyre::config::Theme::new()
        } else {
            color_eyre::config::Theme::default()
        })
        .into_hooks()
        .1
        .install()
        .unwrap();

    gucs::init();

    use tracing_subscriber::{
        layer::SubscriberExt,
        util::SubscriberInitExt,
        EnvFilter,
    };
    let filter_layer = EnvFilter::try_new(gucs::tracing_filters()).expect("Invalid tracing filters set");
    let format_layer = tracing_subscriber::fmt::Layer::new()
        .with_ansi(false)
        .with_writer(|| logging::PgxNoticeWriter::<true>)
        .without_time()
        .pretty();
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(format_layer)
        .try_init().expect("Could not initialize tracing registry");

    plrust::init();
}

/// `pgx` doesn't know how to declare a CREATE FUNCTION statement for a function
/// whose only argument is a `pg_sys::FunctionCallInfo`, so we gotta do that ourselves.
#[pg_extern(sql = "\
CREATE OR REPLACE FUNCTION plrust_call_handler() RETURNS language_handler
    LANGUAGE c AS 'MODULE_PATHNAME', '@FUNCTION_NAME@';\
")]
#[tracing::instrument]
unsafe fn plrust_call_handler(fcinfo: pg_sys::FunctionCallInfo) -> pg_sys::Datum {
    match plrust_call_handler_inner(fcinfo) {
        Ok(datum) => datum,
        // Panic into the pgx guard.
        Err(e) => panic!("{:?}", e),
    }
}

unsafe fn plrust_call_handler_inner(
    fcinfo: pg_sys::FunctionCallInfo,
) -> eyre::Result<pg_sys::Datum> {
    let fn_oid = fcinfo
        .as_ref()
        .ok_or(PlRustError::FunctionCallInfoWasNone)?
        .flinfo
        .as_ref()
        .ok_or(PlRustError::FnOidWasNone)?
        .fn_oid;
    plrust::execute(&fn_oid, &fcinfo)
}

#[pg_extern]
#[tracing::instrument]
unsafe fn plrust_validator(fn_oid: pg_sys::Oid, fcinfo: pg_sys::FunctionCallInfo) {
    match plrust_validator_inner(fn_oid, fcinfo) {
        Ok(()) => (),
        // Panic into the pgx guard.
        Err(e) => panic!("{:?}", e),
    }
}

unsafe fn plrust_validator_inner(
    fn_oid: pg_sys::Oid,
    fcinfo: pg_sys::FunctionCallInfo,
) -> eyre::Result<()> {
    tracing::error!("Oh no!!!!");
    tracing::info!("Oh yes?!?!");
    
    let fcinfo = PgBox::from_pg(fcinfo);
    let flinfo = PgBox::from_pg(fcinfo.flinfo);
    if !pg_sys::CheckFunctionValidatorAccess(
        flinfo.fn_oid,
        pg_getarg(fcinfo.as_ptr(), 0).ok_or(PlRustError::PgGetArgWasNone(fn_oid, 0))?,
    ) {
        return Ok(());
    }

    plrust::unload(&fn_oid)?;

    // NOTE:  We purposely ignore the `check_function_bodies` GUC for compilation as we need to
    // compile the function when it's created to avoid locking during function execution
    let _path = plrust::compile(fn_oid)?;

    Ok(())
}

extension_sql!(
    "\
CREATE LANGUAGE plrust
    HANDLER plrust.plrust_call_handler
    VALIDATOR plrust.plrust_validator;
    
COMMENT ON LANGUAGE plrust IS 'PL/rust procedural language';\
",
    name = "language_handler",
    requires = [plrust_call_handler, plrust_validator]
);
