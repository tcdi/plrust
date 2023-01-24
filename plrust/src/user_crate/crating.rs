/*
Copyright 2021-2023 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

use crate::pgproc::PgProc;
use crate::{
    user_crate::{parse_source_and_deps, CrateState, CrateVariant, FnVerify},
    PlRustError,
};
use eyre::WrapErr;
use pgx::{pg_sys, PgOid};
use quote::quote;
use std::path::Path;

impl CrateState for FnCrating {}

/// Entry point into the FSM for new functions
///
/// - Requires: PL/Rust source input
/// - Produces: a provisioned Cargo crate directory
#[must_use]
pub(crate) struct FnCrating {
    pg_proc_xmin: pg_sys::TransactionId,
    db_oid: pg_sys::Oid,
    fn_oid: pg_sys::Oid,
    user_dependencies: toml::value::Table,
    user_code: syn::Block,
    variant: CrateVariant,
}

impl FnCrating {
    #[cfg(any(test, feature = "pg_test"))]
    pub(crate) fn for_tests(
        pg_proc_xmin: pg_sys::TransactionId,
        db_oid: pg_sys::Oid,
        fn_oid: pg_sys::Oid,
        user_deps: toml::value::Table,
        user_code: syn::Block,
        variant: CrateVariant,
    ) -> Self {
        Self {
            pg_proc_xmin,
            db_oid,
            fn_oid,
            user_dependencies: user_deps.into(),
            user_code,
            variant,
        }
    }

    #[tracing::instrument(level = "debug")]
    pub(crate) unsafe fn try_from_fn_oid(
        db_oid: pg_sys::Oid,
        fn_oid: pg_sys::Oid,
    ) -> eyre::Result<Self> {
        let meta = PgProc::new(fn_oid)?;
        let pg_proc_xmin = meta.xmin();
        let (user_code, user_dependencies) = parse_source_and_deps(&meta.prosrc())?;

        let variant = match meta.prorettype() == pg_sys::TRIGGEROID {
            true => CrateVariant::trigger(),
            false => {
                let argnames = meta.proargnames();
                let argtypes = meta.proargtypes();

                // we must have the same number of argument names and argument types.  It's seemingly
                // impossible that we never would, but lets make sure as it's an invariant from this
                // point forward
                assert_eq!(argnames.len(), argtypes.len());

                let argument_oids_and_names = argtypes
                    .into_iter()
                    .map(|oid| PgOid::from(oid))
                    .zip(argnames.into_iter())
                    .collect();

                CrateVariant::function(
                    argument_oids_and_names,
                    PgOid::from(meta.prorettype()),
                    meta.proretset(),
                    meta.proisstrict(),
                )?
            }
        };

        Ok(Self {
            pg_proc_xmin,
            db_oid,
            fn_oid,
            user_code,
            user_dependencies,
            variant,
        })
    }
    pub(crate) fn crate_name(&self) -> String {
        let mut _crate_name = crate::plrust::crate_name(self.db_oid, self.fn_oid);
        #[cfg(any(
            all(target_os = "macos", target_arch = "x86_64"),
            feature = "force_enable_x86_64_darwin_generations"
        ))]
        {
            let next = crate::generation::next_generation(&_crate_name, true).unwrap_or_default();
            _crate_name.push_str(&format!("_{}", next));
        }
        _crate_name
    }

    /// Generates the lib.rs to write
    #[tracing::instrument(level = "debug", skip_all, fields(db_oid = %self.db_oid, fn_oid = %self.fn_oid))]
    pub(crate) fn lib_rs(&self) -> eyre::Result<syn::File> {
        let crate_name = self.crate_name();
        let symbol_ident = proc_macro2::Ident::new(&crate_name, proc_macro2::Span::call_site());
        tracing::trace!(symbol_name = %crate_name, "Generating `lib.rs` for validation step");

        let user_code = &self.user_code;
        let user_fn: syn::ItemFn = match &self.variant {
            CrateVariant::Function {
                ref arguments,
                ref return_type,
                ..
            } => syn::parse2(quote! {
                fn #symbol_ident(
                    #( #arguments ),*
                ) -> #return_type
                #user_code
            })
            .wrap_err("Parsing generated user function")?,
            CrateVariant::Trigger => syn::parse2(quote! {
                fn #symbol_ident(
                    trigger: &::pgx::PgTrigger,
                ) -> ::core::result::Result<
                    ::pgx::heap_tuple::PgHeapTuple<'_, impl ::pgx::WhoAllocated>,
                    Box<dyn std::error::Error>,
                > #user_code
            })
            .wrap_err("Parsing generated user trigger")?,
        };
        let opened = unsafe_mod(user_fn.clone(), &self.variant)?;
        let forbidden = safe_mod(user_fn)?;

        compose_lib_from_mods([opened, forbidden])
    }

    #[tracing::instrument(level = "debug", skip_all, fields(db_oid = %self.db_oid, fn_oid = %self.fn_oid))]
    pub(crate) fn cargo_toml(&self) -> eyre::Result<toml::value::Table> {
        let major_version = pgx::pg_sys::get_pg_major_version_num();
        let version_feature = format!("pgx/pg{major_version}");
        let crate_name = self.crate_name();

        tracing::trace!(
            crate_name = %crate_name,
            user_dependencies = ?self.user_dependencies.keys().cloned().collect::<Vec<String>>(),
            "Generating `Cargo.toml`"
        );

        let cargo_toml = cargo_toml_template(&crate_name, &version_feature);
        match cargo_toml {
            toml::Value::Table(mut cargo_manifest) => {
                // We have to add the user deps now before we return it.
                match cargo_manifest.entry("dependencies") {
                    toml::value::Entry::Occupied(ref mut occupied) => match occupied.get_mut() {
                        toml::Value::Table(dependencies) => {
                            for (user_dep_name, user_dep_version) in &self.user_dependencies {
                                dependencies
                                    .insert(user_dep_name.clone(), user_dep_version.clone());
                            }
                        }
                        _ => {
                            return Err(PlRustError::GeneratingCargoToml)
                                .wrap_err("Getting `[dependencies]` as table")?
                        }
                    },
                    _ => {
                        return Err(PlRustError::GeneratingCargoToml)
                            .wrap_err("Getting `[dependencies]`")?
                    }
                };

                match std::env::var("PLRUST_EXPERIMENTAL_CRATES") {
                    Err(_) => (),
                    Ok(path) => {
                        match cargo_manifest
                            .entry("patch")
                            .or_insert(toml::Value::Table(Default::default()))
                            .as_table_mut()
                            .unwrap() // infallible
                            .entry("crates-io")
                        {
                            entry @ toml::value::Entry::Vacant(_) => {
                                let mut pgx_table = toml::value::Table::new();
                                pgx_table
                                    .insert("path".into(), toml::Value::String(path.to_string()));
                                let mut crates_io_table = toml::value::Table::new();
                                crates_io_table.insert("pgx".into(), toml::Value::Table(pgx_table));
                                entry.or_insert(toml::Value::Table(crates_io_table));
                            }
                            _ => {
                                return Err(PlRustError::GeneratingCargoToml).wrap_err(
                                    "Setting `[patch]`, already existed (and wasn't expected to)",
                                )?
                            }
                        }
                    }
                };

                Ok(cargo_manifest)
            }
            _ => {
                return Err(PlRustError::GeneratingCargoToml)
                    .wrap_err("Getting `Cargo.toml` as table")?
            }
        }
    }
    /// Provision into a given folder and return the crate directory.
    #[tracing::instrument(level = "debug", skip_all, fields(db_oid = %self.db_oid, fn_oid = %self.fn_oid, parent_dir = %parent_dir.display()))]
    pub(crate) fn provision(&self, parent_dir: &Path) -> eyre::Result<FnVerify> {
        let crate_name = self.crate_name();
        let crate_dir = parent_dir.join(&crate_name);
        let src_dir = crate_dir.join("src");
        std::fs::create_dir_all(&src_dir).wrap_err(
            "Could not create crate directory in configured `plrust.work_dir` location",
        )?;

        let lib_rs = self.lib_rs()?;
        let lib_rs_path = src_dir.join("lib.rs");
        std::fs::write(&lib_rs_path, &prettyplease::unparse(&lib_rs))
            .wrap_err("Writing generated `lib.rs`")?;

        let cargo_toml = self.cargo_toml()?;
        let cargo_toml_path = crate_dir.join("Cargo.toml");
        std::fs::write(
            &cargo_toml_path,
            &toml::to_string(&cargo_toml).wrap_err("Stringifying generated `Cargo.toml`")?,
        )
        .wrap_err("Writing generated `Cargo.toml`")?;

        Ok(FnVerify::new(
            self.pg_proc_xmin,
            self.db_oid,
            self.fn_oid,
            crate_name,
            crate_dir,
        ))
    }
}

/// Throw all the libs into this, we will write this once.
fn compose_lib_from_mods<const N: usize>(modules: [syn::ItemMod; N]) -> eyre::Result<syn::File> {
    let mut skeleton: syn::File = syn::parse2(quote! {
        #![deny(unsafe_op_in_unsafe_fn)]
    })
    .wrap_err("Generating lib skeleton")?;

    for module in modules {
        skeleton.items.push(module.into());
    }
    Ok(skeleton)
}

/// Used by both the unsafe and safe module.
pub(crate) fn shared_imports() -> syn::ItemUse {
    syn::parse_quote!(
        // we (plrust + pgx) fully qualify all pgx imports with `::pgx`, so if the user's function
        // doesn't use any other pgx items we don't want a compiler warning
        #[allow(unused_imports)]
        use pgx::prelude::*;
    )
}

pub(crate) fn cargo_toml_template(crate_name: &str, version_feature: &str) -> toml::Value {
    toml::toml! {
        [package]
        edition = "2021"
        name = crate_name
        version = "0.0.0"

        [features]
        default = [version_feature]

        [lib]
        crate-type = ["cdylib"]

        [dependencies]
        pgx =  { git = "https://github.com/tcdi/plrust", branch = "main", package = "trusted-pgx" }

        /* User deps added here */

        [profile.release]
        debug-assertions = true
        codegen-units = 1_usize
        lto = "fat"
        opt-level = 3_usize
        panic = "unwind"
    }
}

fn unsafe_mod(mut called_fn: syn::ItemFn, variant: &CrateVariant) -> eyre::Result<syn::ItemMod> {
    let imports = shared_imports();

    match variant {
        CrateVariant::Function { .. } => {
            called_fn.attrs.push(syn::parse_quote! {
                #[pg_extern]
            });
        }
        CrateVariant::Trigger => {
            called_fn.attrs.push(syn::parse_quote! {
                #[pg_trigger]
            });
        }
    };

    // Use pub mod so that symbols inside are found, opened, and called
    syn::parse2(quote! {
        pub mod opened {
            #imports

            #called_fn
        }
    })
    .wrap_err("Could not create opened module")
}

fn safe_mod(bare_fn: syn::ItemFn) -> eyre::Result<syn::ItemMod> {
    let imports = shared_imports();
    // Hello from the futurepast!
    // The only situation in which you should be removing this
    // `#![forbid(unsafe_code)]` is if you are moving the forbid
    // command somewhere else  or reconfiguring PL/Rust to also
    // allow it to be run in a fully "Untrusted PL/Rust" mode.
    // This enables the code checking not only for `unsafe {}`
    // but also "unsafe attributes" which are considered unsafe
    // but don't have the `unsafe` token.
    syn::parse2(quote! {
        mod forbidden {
            #![forbid(unsafe_code)]
            #imports

            #bare_fn
        }
    })
    .wrap_err("Could not create forbidden module")
}

#[cfg(any(test, feature = "pg_test"))]
#[pgx::pg_schema]
mod tests {
    use super::*;
    use pgx::*;
    use syn::parse_quote;

    #[pg_test]
    fn strict_string() {
        fn wrapped() -> eyre::Result<()> {
            let pg_proc_xmin = 0 as pg_sys::TransactionId;
            let fn_oid = pg_sys::Oid::INVALID;
            let db_oid = unsafe { pg_sys::MyDatabaseId };

            let variant = {
                let argument_oids_and_names =
                    vec![(PgOid::from(PgBuiltInOids::TEXTOID.value()), None)];
                let return_oid = PgOid::from(PgBuiltInOids::TEXTOID.value());
                let is_strict = true;
                let return_set = false;
                CrateVariant::function(argument_oids_and_names, return_oid, return_set, is_strict)?
            };
            let user_deps = toml::value::Table::default();
            let user_code = syn::parse2(quote! {
                { Some(arg0.to_string()) }
            })?;

            let generated =
                FnCrating::for_tests(pg_proc_xmin, db_oid, fn_oid, user_deps, user_code, variant);

            let crate_name = crate::plrust::crate_name(db_oid, fn_oid);
            #[cfg(any(
                all(target_os = "macos", target_arch = "x86_64"),
                feature = "force_enable_x86_64_darwin_generations"
            ))]
            let crate_name = {
                let mut crate_name = crate_name;
                let (latest, _path) =
                    crate::generation::latest_generation(&crate_name, true).unwrap_or_default();

                crate_name.push_str(&format!("_{}", latest));
                crate_name
            };
            let symbol_ident = proc_macro2::Ident::new(&crate_name, proc_macro2::Span::call_site());

            let generated_lib_rs = generated.lib_rs()?;
            let imports = shared_imports();
            let bare_fn: syn::ItemFn = syn::parse2(quote! {
                fn #symbol_ident(arg0: &str) -> ::std::result::Result<Option<String>, Box<dyn ::std::error::Error>> {
                    Some(arg0.to_string())
                }
            })?;
            let fixture_lib_rs = parse_quote! {
                #![deny(unsafe_op_in_unsafe_fn)]
                pub mod opened {
                    #imports

                    #[pg_extern]
                    #bare_fn
                }

                mod forbidden {
                    #![forbid(unsafe_code)]
                    #imports

                    #bare_fn
                }
            };
            assert_eq!(
                prettyplease::unparse(&generated_lib_rs),
                prettyplease::unparse(&fixture_lib_rs),
                "Generated `lib.rs` differs from test (after formatting)",
            );
            Ok(())
        }
        wrapped().unwrap()
    }

    #[pg_test]
    fn non_strict_integer() {
        fn wrapped() -> eyre::Result<()> {
            let pg_proc_xmin = 0 as pg_sys::TransactionId;
            let fn_oid = pg_sys::Oid::INVALID;
            let db_oid = unsafe { pg_sys::MyDatabaseId };

            let variant = {
                let argument_oids_and_names = vec![(
                    PgOid::from(PgBuiltInOids::INT4OID.value()),
                    Some("val".into()),
                )];
                let return_oid = PgOid::from(PgBuiltInOids::INT8OID.value());
                let is_strict = false;
                let return_set = false;
                CrateVariant::function(argument_oids_and_names, return_oid, return_set, is_strict)?
            };
            let user_deps = toml::value::Table::default();
            let user_code = syn::parse2(quote! {
                { val.map(|v| v as i64) }
            })?;

            let generated =
                FnCrating::for_tests(pg_proc_xmin, db_oid, fn_oid, user_deps, user_code, variant);

            let crate_name = crate::plrust::crate_name(db_oid, fn_oid);
            #[cfg(any(
                all(target_os = "macos", target_arch = "x86_64"),
                feature = "force_enable_x86_64_darwin_generations"
            ))]
            let crate_name = {
                let mut crate_name = crate_name;
                let (latest, _path) =
                    crate::generation::latest_generation(&crate_name, true).unwrap_or_default();

                crate_name.push_str(&format!("_{}", latest));
                crate_name
            };
            let symbol_ident = proc_macro2::Ident::new(&crate_name, proc_macro2::Span::call_site());

            let generated_lib_rs = generated.lib_rs()?;
            let imports = shared_imports();
            let bare_fn: syn::ItemFn = syn::parse2(quote! {
                fn #symbol_ident(val: Option<i32>) -> ::std::result::Result<Option<i64>, Box<dyn ::std::error::Error>> {
                    val.map(|v| v as i64)
                }
            })?;
            let fixture_lib_rs = parse_quote! {
                #![deny(unsafe_op_in_unsafe_fn)]
                pub mod opened {
                    #imports

                    #[pg_extern]
                    #bare_fn
                }

                mod forbidden {
                    #![forbid(unsafe_code)]
                    #imports

                    #bare_fn
                }
            };
            assert_eq!(
                prettyplease::unparse(&generated_lib_rs),
                prettyplease::unparse(&fixture_lib_rs),
                "Generated `lib.rs` differs from test (after formatting)",
            );
            Ok(())
        }
        wrapped().unwrap()
    }

    #[pg_test]
    fn strict_string_set() {
        fn wrapped() -> eyre::Result<()> {
            let pg_proc_xmin = 0 as pg_sys::TransactionId;
            let fn_oid = pg_sys::Oid::INVALID;
            let db_oid = unsafe { pg_sys::MyDatabaseId };

            let variant = {
                let argument_oids_and_names = vec![(
                    PgOid::from(PgBuiltInOids::TEXTOID.value()),
                    Some("val".into()),
                )];
                let return_oid = PgOid::from(PgBuiltInOids::TEXTOID.value());
                let is_strict = true;
                let return_set = true;
                CrateVariant::function(argument_oids_and_names, return_oid, return_set, is_strict)?
            };
            let user_deps = toml::value::Table::default();
            let user_code = syn::parse2(quote! {
                { Ok(Some(std::iter::repeat(val).take(5))) }
            })?;

            let generated =
                FnCrating::for_tests(pg_proc_xmin, db_oid, fn_oid, user_deps, user_code, variant);

            let crate_name = crate::plrust::crate_name(db_oid, fn_oid);
            #[cfg(any(
                all(target_os = "macos", target_arch = "x86_64"),
                feature = "force_enable_x86_64_darwin_generations"
            ))]
            let crate_name = {
                let mut crate_name = crate_name;
                let (latest, _path) =
                    crate::generation::latest_generation(&crate_name, true).unwrap_or_default();

                crate_name.push_str(&format!("_{}", latest));
                crate_name
            };
            let symbol_ident = proc_macro2::Ident::new(&crate_name, proc_macro2::Span::call_site());

            let generated_lib_rs = generated.lib_rs()?;
            let imports = shared_imports();
            let bare_fn: syn::ItemFn = syn::parse2(quote! {
                fn #symbol_ident(val: &str) -> ::std::result::Result<Option<::pgx::iter::SetOfIterator<Option<String>>>, Box<dyn ::std::error::Error>> {
                    Ok(Some(std::iter::repeat(val).take(5)))
                }
            })?;
            let fixture_lib_rs = parse_quote! {
                #![deny(unsafe_op_in_unsafe_fn)]
                pub mod opened {
                    #imports

                    #[pg_extern]
                    #bare_fn
                }

                mod forbidden {
                    #![forbid(unsafe_code)]
                    #imports

                    #bare_fn
                }
            };
            assert_eq!(
                prettyplease::unparse(&generated_lib_rs),
                prettyplease::unparse(&fixture_lib_rs),
                "Generated `lib.rs` differs from test (after formatting)",
            );
            Ok(())
        }
        wrapped().unwrap()
    }

    #[pg_test]
    fn trigger() {
        fn wrapped() -> eyre::Result<()> {
            let pg_proc_xmin = 0 as pg_sys::TransactionId;
            let fn_oid = pg_sys::Oid::INVALID;
            let db_oid = unsafe { pg_sys::MyDatabaseId };

            let variant = CrateVariant::trigger();
            let user_deps = toml::value::Table::default();
            let user_code = syn::parse2(quote! {
                { Ok(trigger.current().unwrap().into_owned()) }
            })?;

            let generated =
                FnCrating::for_tests(pg_proc_xmin, db_oid, fn_oid, user_deps, user_code, variant);

            let crate_name = crate::plrust::crate_name(db_oid, fn_oid);
            #[cfg(any(
                all(target_os = "macos", target_arch = "x86_64"),
                feature = "force_enable_x86_64_darwin_generations"
            ))]
            let crate_name = {
                let mut crate_name = crate_name;
                let (latest, _path) =
                    crate::generation::latest_generation(&crate_name, true).unwrap_or_default();

                crate_name.push_str(&format!("_{}", latest));
                crate_name
            };
            let symbol_ident = proc_macro2::Ident::new(&crate_name, proc_macro2::Span::call_site());

            let generated_lib_rs = generated.lib_rs()?;
            let imports = shared_imports();
            let bare_fn: syn::ItemFn = syn::parse2(quote! {
                fn #symbol_ident(
                    trigger: &::pgx::PgTrigger,
                ) -> ::core::result::Result<
                    ::pgx::heap_tuple::PgHeapTuple<'_, impl ::pgx::WhoAllocated>,
                    Box<dyn std::error::Error>,
                > {
                    Ok(trigger.current().unwrap().into_owned())
                }
            })?;
            let fixture_lib_rs = parse_quote! {
                #![deny(unsafe_op_in_unsafe_fn)]
                pub mod opened {
                    #imports

                    #[pg_trigger]
                    #bare_fn
                }

                mod forbidden {
                    #![forbid(unsafe_code)]
                    #imports

                    #bare_fn
                }
            };
            assert_eq!(
                prettyplease::unparse(&generated_lib_rs),
                prettyplease::unparse(&fixture_lib_rs),
                "Generated `lib.rs` differs from test (after formatting)",
            );
            Ok(())
        }
        wrapped().unwrap()
    }
}
