use std::borrow::Borrow;
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};
use std::ops::{Deref, DerefMut};

use proc_macro2::{Ident, Span, TokenStream};
use quote::{ToTokens, TokenStreamExt};
use serde::{Deserialize, Serialize};

use crate::gucs::{PLRUST_COMPILE_LINTS, PLRUST_REQUIRED_LINTS};

#[derive(Debug, Clone, Serialize, Deserialize, Ord, PartialOrd, PartialEq, Eq, Hash)]
pub(crate) struct Lint(String);

#[derive(Debug, Clone, Serialize, Deserialize, Ord, PartialOrd, PartialEq, Eq, Hash)]
pub(crate) struct LintSet(BTreeSet<Lint>);

impl Deref for LintSet {
    type Target = BTreeSet<Lint>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for LintSet {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Display for LintSet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.iter().collect::<Vec<String>>().join(", "))
    }
}

impl ToTokens for LintSet {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        for lint in self.iter() {
            lint.to_tokens(tokens);
        }
    }
}

impl FromIterator<Lint> for LintSet {
    fn from_iter<T: IntoIterator<Item = Lint>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<'a> FromIterator<&'a Lint> for Vec<String> {
    fn from_iter<T: IntoIterator<Item = &'a Lint>>(iter: T) -> Self {
        iter.into_iter().map(|l| l.to_string()).collect()
    }
}

impl Display for Lint {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Deref for Lint {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Borrow<str> for Lint {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl From<&str> for Lint {
    fn from(value: &str) -> Self {
        Lint(value.to_string())
    }
}

impl ToTokens for Lint {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let lint = Ident::new(&self.0, Span::call_site());
        tokens.append_all(quote::quote!(#![forbid(#lint)]))
    }
}

/// Use the set of lints configured via the `plrust.compile_lints` GUC
pub(crate) fn compile_lints() -> LintSet {
    PLRUST_COMPILE_LINTS
        .get()
        .unwrap_or_default()
        .split(',')
        .filter(|x| !x.is_empty())
        .map(|s| s.trim().into())
        .collect()
}

/// Enumerates the set of lints that are required to have been applied to a plrust function during
/// compilation.  These should be squared against the metadata we have for each function before
/// they're dlopen()'d
pub(crate) fn required_lints() -> LintSet {
    let filter_map = |s: &str| {
        let trim = s.trim();
        if !trim.is_empty() {
            Some(trim.into())
        } else {
            None
        }
    };

    // if a `PLRUST_REQUIRED_LINTS` environment variable exists, we always use it, no questions asked
    //
    // we do this because we want the person with root on the box, if they so desire, to require that
    // PL/Rust functions we're going to execute have the properties they require for their environment
    let mut forced = std::env::var("PLRUST_REQUIRED_LINTS")
        .unwrap_or_default()
        .split(',')
        .filter_map(filter_map)
        .collect::<LintSet>();

    // maybe it's an unfounded belief, but perhaps the person with root is different than the person
    // that can edit postgresql.conf.  So we union what might be in our environment with with
    // whatever might be configured in postgresql.conf
    let mut configured = PLRUST_REQUIRED_LINTS
        .get()
        .unwrap_or_else(|| PLRUST_COMPILE_LINTS.get().unwrap_or_default())
        .split(',')
        .filter_map(filter_map)
        .collect::<LintSet>();
    configured.append(&mut forced);
    configured
}
