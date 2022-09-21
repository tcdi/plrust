use crate::{user_crate::oid_to_syn_type, PlRustError};
use eyre::WrapErr;
use pgx::PgOid;
use proc_macro2::{Ident, Span};
use quote::quote;
use std::collections::HashMap;

#[must_use]
pub(crate) enum CrateVariant {
    Function {
        arguments: HashMap<(PgOid, Option<String>), syn::FnArg>,
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
        arguement_oids_and_names: Vec<(PgOid, Option<String>)>,
        return_oid: PgOid,
        return_set: bool,
        is_strict: bool,
    ) -> eyre::Result<Self> {
        let mut arguments = HashMap::default();
        for (idx, (argument_oid, maybe_argument_name)) in
            arguement_oids_and_names.into_iter().enumerate()
        {
            let rust_type: syn::Type = {
                let bare = oid_to_syn_type(&argument_oid, false)?;
                match is_strict {
                    true => bare,
                    false => syn::parse2(quote! {
                        Option<#bare>
                    })
                    .wrap_err("Wrapping argument type")?,
                }
            };

            let argument_name = match &maybe_argument_name {
                Some(argument_name) => Ident::new(&argument_name.clone(), Span::call_site()),
                None => Ident::new(&format!("arg{}", idx), Span::call_site()),
            };
            let rust_pat_type: syn::FnArg = syn::parse2(quote! {
                #argument_name: #rust_type
            })
            .map_err(PlRustError::Parse)
            .wrap_err("Making argument pattern type")?;
            arguments.insert((argument_oid, maybe_argument_name), rust_pat_type);
        }

        let return_type: syn::Type = {
            let bare = oid_to_syn_type(&return_oid, true)?;
            match return_set {
                true => syn::parse2(quote! { Option<::pgx::iter::SetOfIterator<Option<#bare>>> })
                    .wrap_err("Wrapping return type")?,
                false => syn::parse2(quote! { Option<#bare> }).wrap_err("Wrapping return type")?,
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
