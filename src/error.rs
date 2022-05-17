#[derive(thiserror::Error, Debug)]
pub enum PlRustError {
    #[error("Failed pg_sys::CheckFunctionValidatorAccess")]
    CheckFunctionValidatorAccess,
    #[error("pgx::pg_sys::FunctionCallInfo was Null")]
    NullFunctionCallInfo,
    #[error("pgx::pg_sys::FmgrInfo was Null")]
    NullFmgrInfo,
    #[error("The Procedure Tuple was NULL")]
    NullProcTuple,
    #[error("The source code of the function was NULL")]
    NullSourceCode,
    #[error("libloading error: {0}")]
    LibLoading(#[from] libloading::Error),
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    #[error("Generation error (Mac OS x86_64 specific): {0}")]
    Generation(#[from] crate::plrust::generation::Error),
    #[error("Creating crate directory in plrust.work_dir GUC location: {0}")]
    CrateDirectory(std::io::Error),
    #[error("Executing `cargo build`: {0}")]
    CargoBuildExec(std::io::Error),
    #[error("`cargo build` failed")]
    CargoBuildFail,
    #[error("Produced shared object not found")]
    SharedObjectNotFound,
    #[error("Cargo output was not UTF-8: {0}")]
    CargoOutputNotUtf8(std::string::FromUtf8Error),
    #[error("Creating source directory: {0}")]
    CreatingSourceDirectory(std::io::Error),
    #[error("Writing source code to `src/lib.rs`: {0}")]
    WritingLibRs(std::io::Error),
    #[error("Writing `Cargo.toml`: {0}")]
    WritingCargoToml(std::io::Error),
    #[error("Unsupported SQL type OID: {0}")]
    UnsupportedSqlType(pgx::pg_sys::Oid),
    #[error("Function `{0}` was not a PL/Rust function")]
    NotPlRustFunction(pgx::pg_sys::Oid),
}
