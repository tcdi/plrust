use std::{
    fmt::{Display, Formatter},
    io,
    path::PathBuf,
    process::ExitStatus,
};

#[derive(thiserror::Error, Debug)]
pub enum PlRustError {
    #[error("WASM guest error: {0}")]
    Guest(#[from] crate::guest::Error),
    #[error("WASM guest experienced a trap: {0}")]
    Trap(#[from] wasmtime::Trap),
    #[error("WASM WASI error: {0}")]
    Wasi(#[from] wasmtime_wasi::Error),
    #[error("Failed to parse `cargo build` messages: {0}")]
    CargoMessageParse(io::Error),
    #[error("Module generation failed on file {0}: {1}")]
    ModuleFileGeneration(PathBuf, io::Error),
    #[error("Module build failed to execute: {0}")]
    ModuleBuildExecution(io::Error),
    #[error("`cargo build` failed with code {0}")]
    ModuleExitNonZero(ExitStatus),
    #[error("Module not found: {0}")]
    ModuleNotFound(String),
    #[error("Failed to relocate compiled wasm artifact: {0}")]
    ModuleRelocation(io::Error),
    #[error("Failed to cleanup build directory {0}: {1}")]
    Cleanup(PathBuf, io::Error),
    #[error("FunctionCallInfo was None")]
    FunctionCallInfoWasNone,
    #[error("FunctionCallInfo provided fn_oid was None")]
    FnOidWasNone,
    #[error("pg_getarg on fn_oid {0} at index {1} was None")]
    PgGetArgWasNone(pgx::pg_sys::Oid, u64),
}

// Guest

impl Display for crate::guest::Error {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            crate::guest::Error::ConversionError(e) => write!(f, "{}", e),
            crate::guest::Error::CoerceError(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for crate::guest::Error {}

impl std::error::Error for crate::guest::ConversionError {}

impl Display for crate::guest::ConversionError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "Could not turn value {} into a {}", self.value, self.into)
    }
}

// Host

impl Display for crate::host::Error {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            crate::host::Error::ConversionError(e) => write!(f, "{}", e),
            crate::host::Error::CoerceError(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for crate::host::Error {}

impl std::error::Error for crate::host::ConversionError {}

impl Display for crate::host::ConversionError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "Could not turn value {} into a {}", self.value, self.into)
    }
}
