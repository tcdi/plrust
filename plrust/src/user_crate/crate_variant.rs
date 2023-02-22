/*
Copyright 2021-2023 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

use crate::pgproc::ProArgMode;
use crate::{user_crate::oid_to_syn_type, PlRustError};
use eyre::WrapErr;
use pgx::{pg_sys, PgOid};
use proc_macro2::{Ident, Span};
use quote::quote;

/// What kind of PL/Rust function must be built

/// Includes arguments and return type, if applicable
/// Used to create the source code that is built
#[must_use]
#[derive(Clone)]
pub(crate) enum CrateVariant {
    Function {
        arguments: Vec<syn::FnArg>,
        return_type: syn::Type,
        #[allow(dead_code)] // For debugging
        return_oid: PgOid,
        #[allow(dead_code)] // For debugging
        return_set: bool,
        #[allow(dead_code)] // For debugging
        is_strict: bool,
    },
    Trigger,
}

impl CrateVariant {
    #[tracing::instrument(level = "debug", skip_all)]
    pub(crate) fn function(
        argnames: Vec<Option<String>>,
        argtypes: Vec<pg_sys::Oid>,
        argmodes: Vec<ProArgMode>,
        return_oid: PgOid,
        return_set: bool,
        is_strict: bool,
    ) -> eyre::Result<Self> {
        // we must have the same number of argument names, argument types, and modes  It's seemingly
        // impossible that we never would, but lets make sure as it's an invariant from this
        // point forward
        assert_eq!(
            argnames.len(),
            argtypes.len(),
            "mismatched argument names and types"
        );
        assert_eq!(
            argnames.len(),
            argmodes.len(),
            "mismatched argument names and modes"
        );

        let return_table = return_set && argmodes.contains(&ProArgMode::Table);

        // provide default names for any args that don't have one
        let mut argnames = argnames
            .into_iter()
            .enumerate()
            .map(|(idx, name)| name.unwrap_or_else(|| format!("arg{}", idx)))
            .collect::<Vec<_>>();

        // convert the raw type oids into `PgOid`
        let mut argtypes = argtypes
            .into_iter()
            .map(|oid| PgOid::from(oid))
            .collect::<Vec<_>>();

        let mut tabletypes = Vec::new();
        if return_table {
            // Postgres treats the columns in a RETURNS TABLE(...) statement as arguments of type 't' (table)
            // and we need to separate them from the rest of the arguments
            let mut filtered_argnames = Vec::new();
            let mut filtered_argtypes = Vec::new();

            for ((argmode, argtype), argname) in argmodes
                .into_iter()
                .zip(argtypes.into_iter())
                .zip(argnames.into_iter())
            {
                if argmode == ProArgMode::Table {
                    // remember this 't'able argument type
                    tabletypes.push(argtype);
                } else {
                    filtered_argnames.push(argname);
                    filtered_argtypes.push(argtype);
                }
            }

            // swap in the filtered lists of names and types
            argnames = filtered_argnames;
            argtypes = filtered_argtypes;
        };

        let mut arguments = Vec::new();
        for (type_oid, arg_name) in argtypes.into_iter().zip(argnames.into_iter()) {
            let rust_type: syn::Type = {
                let bare = oid_to_syn_type(&type_oid, false)?;
                match is_strict {
                    true => bare,
                    false => syn::parse2(quote! {
                        Option<#bare>
                    })
                    .wrap_err("Wrapping argument type")?,
                }
            };

            let arg_name = Ident::new(&arg_name, Span::call_site());
            let rust_pat_type: syn::FnArg = syn::parse2(quote! {
                #arg_name: #rust_type
            })
            .map_err(PlRustError::Parse)
            .wrap_err("Making argument pattern type")?;
            arguments.push(rust_pat_type);
        }

        let return_type: syn::Type = {
            let bare = oid_to_syn_type(&return_oid, true)?;
            match return_set {
                true => match return_table {
                    true => {
                        // it's a `RETURNS TABLE(...)`
                        let syntypes = tabletypes
                            .into_iter()
                            .map(|t| oid_to_syn_type(&t, true))
                            .collect::<Result<Vec<_>, _>>()?;
                        syn::parse2(quote! {
                            ::std::result::Result::<Option<::pgx::iter::TableIterator<'a, ( #(Option<#syntypes>),*, ) >>, Box<dyn ::std::error::Error>>
                        }).wrap_err("Wrapping TableIterator return type")?
                    }

                    false => {
                        // it's a `RETURNS SETOF xxx`
                        syn::parse2(quote! { ::std::result::Result<Option<::pgx::iter::SetOfIterator<'a, Option<#bare>>>, Box<dyn ::std::error::Error>> })
                            .wrap_err("Wrapping SetOfIterator return type")?
                    }
                },

                false => {
                    // it's a plain `RETURNS xxx`
                    syn::parse2(quote! { ::std::result::Result<Option<#bare>, Box<dyn ::std::error::Error>> }).wrap_err("Wrapping return type")?
                }
            }
        };

        Ok(Self::Function {
            arguments,
            return_oid,
            return_type,
            return_set,
            is_strict,
        })
    }

    #[tracing::instrument(level = "debug", skip_all)]
    pub(crate) fn trigger() -> Self {
        Self::Trigger
    }
}
