use eyre::WrapErr;
use crate::{user_crate::oid_to_syn_type, PlRustError};
use proc_macro2::{Span, Ident};
use std::collections::HashMap;
use pgx::PgOid;
use quote::quote;

#[must_use]
pub enum CrateVariant {
    Function {
        arguments: HashMap<(PgOid, Option<String>), syn::FnArg>,
        return_oid: PgOid,
        return_type: syn::Type,
        return_set: bool,
        is_strict: bool,
    },
    // Trigger,
}

impl CrateVariant {
    #[tracing::instrument(level = "debug", skip_all)]
    pub fn function(
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
                true => syn::parse2(quote! { Option<impl Iterator<Item=Option<#bare>> + '_> })
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
}