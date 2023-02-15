/*
Portions Copyright 2020-2021 ZomboDB, LLC.
Portions Copyright 2021-2023 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

#![doc = include_str!("../../README.md")]
#![forbid(unsafe_op_in_unsafe_fn)]

#[cfg(all(
    feature = "trusted",
    not(all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    ))
))]
compile_error!("This platform does not support the 'trusted' version of plrust");

cfg_if::cfg_if! {
    if #[cfg(feature = "trusted")] {
        /// We are a trusted language handler.  This will cause plrust user functions to be compiled
        /// with `postgrestd`
        pub(crate) const TRUSTED: bool = true;
    } else {
        /// We are NOT a trusted language handler.  This will cause plrust user functions to be compiled
        /// with the standard Rust `std`
        pub(crate) const TRUSTED: bool = false;
    }
}

mod error;
mod gucs;
mod logging;
mod plrust;

mod user_crate;

mod hooks;
mod pgproc;
mod prosrc;
pub(crate) mod target;

#[cfg(any(test, feature = "pg_test"))]
pub mod tests;

use error::PlRustError;
use pgx::{pg_getarg, prelude::*};

#[cfg(any(test, feature = "pg_test"))]
pub use tests::pg_test;
pgx::pg_module_magic!();

/// This is the default set of lints we apply to PL/Rust user functions, and require of PL/Rust user
/// functions before we'll load and execute them.
///
/// The defaults **can** be changed with the `plrust.compile_lints` and `plrust.required_lints` GUCS
// Hello from the futurepast!
// The only situation in which you should be removing this
// `#![forbid(unsafe_code)]` is if you are moving the forbid
// command somewhere else  or reconfiguring PL/Rust to also
// allow it to be run in a fully "Untrusted PL/Rust" mode.
// This enables the code checking not only for `unsafe {}`
// but also "unsafe attributes" which are considered unsafe
// but don't have the `unsafe` token.
const DEFAULT_LINTS: &'static str = "plrust_extern_blocks, plrust_lifetime_parameterized_traits, implied_bounds_entailment, plrust_filesystem_macros, unsafe_code";

#[pg_guard]
fn _PG_init() {
    // Must be loaded with shared_preload_libraries
    unsafe {
        // SAFETY:  We're required to be loaded as a "shared preload library", and Postgres will
        // set this static to true before trying to load any of those libraries
        if !pg_sys::process_shared_preload_libraries_in_progress {
            ereport!(
                ERROR,
                PgSqlErrorCode::ERRCODE_OBJECT_NOT_IN_PREREQUISITE_STATE,
                "plrust must be loaded via shared_preload_libraries"
            );
        }
    }

    color_eyre::config::HookBuilder::default()
        .theme(color_eyre::config::Theme::new())
        .into_hooks()
        .1
        .install()
        .unwrap();

    gucs::init();
    hooks::init();

    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

    let filter_layer = EnvFilter::builder()
        .with_default_directive(gucs::tracing_level().into())
        .from_env()
        .expect("Error parsing default log level");

    let error_layer = tracing_error::ErrorLayer::default();

    let format_layer = tracing_subscriber::fmt::Layer::new()
        .with_ansi(false)
        .with_writer(|| logging::PgxNoticeWriter::<true>)
        .without_time()
        .pretty();
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(format_layer)
        .with(error_layer)
        .try_init()
        .expect("Could not initialize tracing registry");

    plrust::init();
}

/// `pgx` doesn't know how to declare a CREATE FUNCTION statement for a function
/// whose only argument is a `pg_sys::FunctionCallInfo`, so we gotta do that ourselves.
#[pg_extern(sql = "
CREATE FUNCTION plrust_call_handler() RETURNS language_handler
    LANGUAGE c AS 'MODULE_PATHNAME', '@FUNCTION_NAME@';
")]
#[tracing::instrument(level = "debug")]
unsafe fn plrust_call_handler(fcinfo: pg_sys::FunctionCallInfo) -> pg_sys::Datum {
    unsafe fn plrust_call_handler_inner(
        fcinfo: pg_sys::FunctionCallInfo,
    ) -> eyre::Result<pg_sys::Datum> {
        // SAFETY: these seemingly innocent invocations of `as_ref` are actually `pointer::as_ref`
        // but we should have been given this fcinfo by Postgres, so it should be fine
        let fn_oid = unsafe {
            fcinfo
                .as_ref()
                .ok_or(PlRustError::NullFunctionCallInfo)?
                .flinfo
                .as_ref()
        }
        .ok_or(PlRustError::NullFmgrInfo)?
        .fn_oid;
        let retval = unsafe { plrust::evaluate_function(fn_oid, fcinfo)? };
        Ok(retval)
    }

    // SAFETY: This is more of a "don't call us, we'll call you" situation.
    match unsafe { plrust_call_handler_inner(fcinfo) } {
        Ok(datum) => datum,
        // Panic into the pgx guard.
        Err(err) => panic!("{:?}", err),
    }
}

/// Called by Postgres, not you.
/// # Safety
/// Don't.
#[pg_extern]
#[tracing::instrument(level = "debug")]
unsafe fn plrust_validator(fn_oid: pg_sys::Oid, fcinfo: pg_sys::FunctionCallInfo) {
    unsafe fn plrust_validator_inner(
        fn_oid: pg_sys::Oid,
        fcinfo: pg_sys::FunctionCallInfo,
    ) -> eyre::Result<()> {
        let fcinfo = unsafe { PgBox::from_pg(fcinfo) };
        let flinfo = unsafe { PgBox::from_pg(fcinfo.flinfo) };
        // We were called by Postgres hopefully
        if unsafe {
            !pg_sys::CheckFunctionValidatorAccess(
                flinfo.fn_oid,
                pg_getarg(fcinfo.as_ptr(), 0).unwrap(),
            )
        } {
            return Err(PlRustError::CheckFunctionValidatorAccess)?;
        }

        unsafe { plrust::unload_function(fn_oid) };
        // NOTE:  We purposely ignore the `check_function_bodies` GUC for compilation as we need to
        // compile the function when it's created to avoid locking during function execution
        let output = plrust::compile_function(fn_oid)?;

        // however, we'll use it to decide if we should go ahead and dynamically load our function
        // SAFETY: This should always be set by Postgres.
        if unsafe { pg_sys::check_function_bodies } {
            // it's on, so lets go ahead and load our function
            // plrust::lookup_function(fn_oid);
        }

        // if the compilation had warnings we'll display them
        let stderr =
            String::from_utf8(output.stdout.clone()).expect("`cargo`'s stdout was not UTF-8");
        if stderr.contains("warning: ") {
            pgx::warning!("\n{}", stderr);
        }

        Ok(())
    }

    match unsafe { plrust_validator_inner(fn_oid, fcinfo) } {
        Ok(()) => (),
        // Panic into the pgx guard.
        Err(err) => panic!("{:?}", err),
    }
}

#[cfg(feature = "trusted")]
extension_sql!(
    r#"
CREATE TRUSTED LANGUAGE plrust
    HANDLER plrust.plrust_call_handler
    VALIDATOR plrust.plrust_validator;

COMMENT ON LANGUAGE plrust IS 'Trusted PL/rust procedural language';
"#,
    name = "language_handler",
    requires = [plrust_call_handler, plrust_validator]
);

#[cfg(not(feature = "trusted"))]
extension_sql!(
    r#"
CREATE LANGUAGE plrust
    HANDLER plrust.plrust_call_handler
    VALIDATOR plrust.plrust_validator;

COMMENT ON LANGUAGE plrust IS 'Untrusted PL/Rust procedural language';

DO LANGUAGE plpgsql $$
BEGIN
    RAISE WARNING 'plrust is **NOT** compiled to be a trusted procedural language';
END;
$$;
"#,
    name = "language_handler",
    requires = [plrust_call_handler, plrust_validator]
);
