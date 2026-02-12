/*
Portions Copyright 2021-2025 Technology Concepts & Design, Inc.

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// The capabilities that influence how PL/Rust generates wrapper code for a user function
// NB:  Make sure to add new ones down below to [`FunctionCapabilitySet::default()`]
#[derive(Debug, Clone, Serialize, Deserialize, Ord, PartialOrd, PartialEq, Eq, Hash)]
pub(crate) enum FunctionCapability {
    /// Indicates that `pgrx::Array<'a, T>` should be used instead of `Vec<T>` for mapping
    /// arguments of SQL type `ARRAY[]::T[]`
    ZeroCopyArrays,
}

/// A set of [`FunctionCapability`] which is stored as metadata in the system catalogs
#[derive(Debug, Clone, Serialize, Deserialize, Ord, PartialOrd, PartialEq, Eq, Hash)]
pub(crate) struct FunctionCapabilitySet(BTreeSet<FunctionCapability>);

impl Default for FunctionCapabilitySet {
    /// Creates a default [`FunctionCapabilitySet`] which contains every [`FunctionCapability`]
    /// PL/Rust supports.  
    #[inline]
    fn default() -> Self {
        let mut caps = BTreeSet::default();
        caps.insert(FunctionCapability::ZeroCopyArrays);
        Self(caps)
    }
}

impl FunctionCapabilitySet {
    /// Create a [`FunctionCapabilitySet`] that contains nothing.  This is a convenience method
    /// for backwards compatibility with PL/Rust v1.0.0 which did not have capabilities
    #[inline]
    pub(crate) fn empty() -> Self {
        Self(Default::default())
    }

    /// Does the set contain the [`FunctionCapability::ZeroCopyArrays]` capability?
    #[inline]
    pub fn has_zero_copy_arrays(&self) -> bool {
        self.0.contains(&FunctionCapability::ZeroCopyArrays)
    }
}
