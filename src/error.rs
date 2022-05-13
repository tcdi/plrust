
#[derive(thiserror::Error, Debug)]
pub enum PlRustError {
    #[error("Failed pg_sys::CheckFunctionValidatorAccess")]
    CheckFunctionValidatorAccess,
    #[error("pgx::pg_sys::FunctionCallInfo was Null")]
    NullFunctionCallInfo,
    #[error("pgx::pg_sys::FmgrInfo was Null")]
    NullFmgrInfo,
    #[error("libloading error: {0}")]
    LibLoading(#[from] libloading::Error),
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    #[error("Generation error (Mac OS x86_64 specific): {0}")]
    Generation(#[from] crate::generation::Error),
    #[error("Creating crate directory in plrust.work_dir GUC location: {0}")]
    CrateDirectory(std::io::Error),
    #[error("Executing `cargo build`: {0}")]
    CargoBuildExec(std::io::Error),
    #[error("`cargo build` failed: {0}")]
    CargoBuildFail(String),
    #[error("Produced shared object not found")]
    SharedObjectNotFound,
    #[error("Cargo output was not UTF-8: {0}")]
    CargoOutputNotUtf8(std::string::FromUtf8Error)
}