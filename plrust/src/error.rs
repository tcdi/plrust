/*
Copyright 2021-2023 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

use crate::target::CompilationTarget;
use crate::user_crate::lint::LintSet;

#[derive(thiserror::Error, Debug)]
pub(crate) enum PlRustError {
    #[error("Failed pg_sys::CheckFunctionValidatorAccess")]
    CheckFunctionValidatorAccess,
    #[error("pgrx::pg_sys::FunctionCallInfo was Null")]
    NullFunctionCallInfo,
    #[error("pgrx::pg_sys::FmgrInfo was Null")]
    NullFmgrInfo,
    #[error("libloading error: {0}")]
    LibLoading(#[from] libloading::Error),
    #[error("`cargo build` failed")]
    CargoBuildFail,
    #[error("Generating `Cargo.toml`")]
    GeneratingCargoToml,
    #[error("Function `{0}` does not exist")]
    NoSuchFunction(pgrx::pg_sys::Oid),
    #[error("Oid `{0}` was not mappable to a Rust type")]
    NoOidToRustMapping(pgrx::pg_sys::Oid),
    #[error("Generated Rust type (`{1}`) for `{0}` was unparsable: {2}")]
    ParsingRustMapping(pgrx::pg_sys::Oid, String, syn::Error),
    #[error("Parsing `[code]` block: {0}")]
    ParsingCodeBlock(syn::Error),
    #[error("Parsing error at span `{:?}`", .0.span())]
    Parse(#[from] syn::Error),
    #[error("Function was not compiled for this host (`{0}`)")]
    FunctionNotCompiledForTarget(CompilationTarget),
    #[error("Function not compiled with required lints: {0}")]
    MissingLints(LintSet),
}
