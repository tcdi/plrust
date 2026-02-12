/*
Portions Copyright 2021-2025 Technology Concepts & Design, Inc.

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE file.
*/
/*
Copyright 2021-2023 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

use crate::error::PlRustError;
use pgrx::{pg_sys, FromDatum, IntoDatum, PgLogLevel, PgRelation, PgSqlErrorCode};
use std::ptr::NonNull;

/// Provides a safe wrapper around a Postgres "SysCache" entry from `pg_catalog.pg_proc`.
pub(crate) struct PgProc {
    inner: NonNull<pg_sys::HeapTupleData>,
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub(crate) enum ProArgMode {
    In,
    Out,
    InOut,
    Variadic,
    Table,
}

impl From<i8> for ProArgMode {
    fn from(value: i8) -> Self {
        match value as u8 {
            b'i' => ProArgMode::In,
            b'o' => ProArgMode::Out,
            b'b' => ProArgMode::InOut,
            b'v' => ProArgMode::Variadic,
            b't' => ProArgMode::Table,

            // there's just no ability to move forward if given a value that we don't know about
            _ => panic!("unrecognized `ProArgMode`: `{}`", value),
        }
    }
}

impl Drop for PgProc {
    fn drop(&mut self) {
        // SAFETY: We have a valid pointer and this just decrements the reference count.
        // This will generally get resolved by the end of the transaction anyways,
        // but Postgres strongly recommends you do not do that.
        unsafe { pg_sys::ReleaseSysCache(self.inner.as_ptr()) }
    }
}

impl PgProc {
    #[inline]
    pub(crate) fn new(pg_proc_oid: pg_sys::Oid) -> std::result::Result<PgProc, PlRustError> {
        unsafe {
            // SAFETY:  SearchSysCache1 will give us a valid HeapTuple or it'll return null.
            // Either way, using NonNull::new()? will make the right decision for us
            let entry = pg_sys::SearchSysCache1(
                pg_sys::SysCacheIdentifier_PROCOID as _,
                pg_proc_oid.into_datum().unwrap(),
            );
            let inner = match NonNull::new(entry) {
                Some(inner) => inner,
                None => return Err(PlRustError::NoSuchFunction(pg_proc_oid)),
            };
            Ok(PgProc { inner })
        }
    }

    pub(crate) fn relation() -> PgRelation {
        unsafe {
            // SAFETY:  [`pg_sys::ProcedureRelationId`] is a compiled-in relation oid, and
            // [`pg_sys::AccessShareLock`] is a valid lock value
            PgRelation::with_lock(pg_sys::ProcedureRelationId, pg_sys::AccessShareLock as _)
        }
    }

    /// Return a copy of the backing [`pg_sys::HeapTupleData`] allocated in the `CurrentMemoryContext`
    #[inline]
    pub(crate) fn heap_tuple(&self) -> *mut pg_sys::HeapTupleData {
        unsafe {
            // SAFETY:  we know that `self.inner` is always a valid [pg_sys::HeapTupleData] pointer
            // because we're the only one that creates it
            pg_sys::heap_copytuple(self.inner.as_ptr())
        }
    }

    #[inline]
    fn xmin(&self) -> pg_sys::TransactionId {
        // SAFETY:  self.inner will be valid b/c that's part of what pg_sys::SearchSysCache1()
        // does for us.  Same is true for t_data
        unsafe {
            self.inner
                .as_ref()
                .t_data
                .as_ref()
                .unwrap_unchecked() // SAFETY: t_data will never be null and `xmin()` is called in a potentially hot path
                .t_choice
                .t_heap
                .t_xmin
        }
    }

    #[inline]
    fn cmin(&self) -> pg_sys::CommandId {
        // SAFETY:  self.inner will be valid b/c that's part of what pg_sys::SearchSysCache1()
        // does for us.  Same is true for t_data
        unsafe {
            self.inner
                .as_ref()
                .t_data
                .as_ref()
                .unwrap_unchecked() // SAFETY: t_data will never be null and `cmin()` is called in a potentially hot path
                .t_choice
                .t_heap
                .t_field3
                .t_cid
        }
    }

    #[inline]
    pub(crate) fn generation_number(&self) -> u64 {
        ((self.xmin() as u64) << 32_u64) | self.cmin() as u64
    }

    pub(crate) fn ctid(&self) -> pg_sys::ItemPointerData {
        unsafe {
            // SAFETY:  self.inner will be valid b/c that's part of what pg_sys::SearchSysCache1()
            // does for us.  Same is true for t_data
            (*self.inner.as_ref().t_data).t_ctid
        }
    }

    pub(crate) fn prolang(&self) -> pg_sys::Oid {
        // SAFETY:  `prolang` has a NOT NULL constraint
        self.get_attr(pg_sys::Anum_pg_proc_prolang).unwrap()
    }

    pub(crate) fn prosrc(&self) -> String {
        // SAFETY:  `prosrc` has a NOT NULL constraint
        self.get_attr(pg_sys::Anum_pg_proc_prosrc).unwrap()
    }

    /// ```
    /// proargmodes char[]
    /// An array of the modes of the function arguments, encoded as i for IN arguments, o for OUT arguments,
    /// b for INOUT arguments, v for VARIADIC arguments, t for TABLE arguments. If all the arguments
    /// are IN arguments, this field will be null. Note that subscripts correspond to positions of
    /// proallargtypes not proargtypes.
    /// ```
    ///
    /// In our case, if all the arguments are `IN` arguments, the returned Vec will have the
    /// corresponding `ProArgModes::In` value in each element.
    pub(crate) fn proargmodes(&self) -> Vec<ProArgMode> {
        self.get_attr::<Vec<i8>>(pg_sys::Anum_pg_proc_proargmodes)
            .unwrap_or_else(|| vec!['i' as i8; self.proargnames().len()])
            .into_iter()
            .map(|mode| ProArgMode::from(mode))
            .collect::<Vec<_>>()
    }

    pub(crate) fn pronargs(&self) -> usize {
        // SAFETY:  `pronargs` has a NOT NULL constraint
        self.get_attr::<i16>(pg_sys::Anum_pg_proc_pronargs).unwrap() as usize
    }

    pub(crate) fn proargnames(&self) -> Vec<syn::Ident> {
        self.get_attr::<Vec<Option<String>>>(pg_sys::Anum_pg_proc_proargnames)
            .unwrap_or_else(|| vec![None; self.pronargs()])
            .into_iter()
            .map(|name| {
                let name = name.unwrap_or_else(|| String::default());

                syn::parse_str::<syn::Ident>(&name)
                    .unwrap_or_else(|_| {
                        static DETAIL:&'static str = "PL/Rust argument names must also be valid Rust identifiers.  Rust's identifier specification can be found at https://doc.rust-lang.org/reference/identifiers.html";
                        if name.is_empty() {
                            pgrx::ereport!(PgLogLevel::ERROR, PgSqlErrorCode::ERRCODE_INVALID_NAME, "PL/Rust does not support unnamed arguments", DETAIL);
                        } else {
                            pgrx::ereport!(PgLogLevel::ERROR, PgSqlErrorCode::ERRCODE_INVALID_NAME, format!("`{name}` is an invalid Rust identifier and cannot be used as an argument name"), DETAIL);
                        }
                        unreachable!()
                    })
            })
            .collect()
    }

    pub(crate) fn proargtypes(&self) -> Vec<pg_sys::Oid> {
        self.get_attr(pg_sys::Anum_pg_proc_proargtypes)
            .unwrap_or_default()
    }

    pub(crate) fn proallargtypes(&self) -> Vec<pg_sys::Oid> {
        self.get_attr(pg_sys::Anum_pg_proc_proallargtypes)
            .unwrap_or_else(|| self.proargtypes())
    }

    pub(crate) fn prorettype(&self) -> pg_sys::Oid {
        // SAFETY:  `prorettype` has a NOT NULL constraint
        self.get_attr(pg_sys::Anum_pg_proc_prorettype).unwrap()
    }

    pub(crate) fn proisstrict(&self) -> bool {
        // SAFETY: 'proisstrict' has a NOT NULL constraint
        self.get_attr(pg_sys::Anum_pg_proc_proisstrict).unwrap()
    }

    pub(crate) fn proretset(&self) -> bool {
        // SAFETY: 'proretset' has a NOT NULL constraint
        self.get_attr(pg_sys::Anum_pg_proc_proretset).unwrap()
    }

    #[inline]
    fn get_attr<T: FromDatum>(&self, attribute: u32) -> Option<T> {
        unsafe {
            // SAFETY:  SysCacheGetAttr will give us what we need to create a Datum of type T,
            // and this PgProc type ensures we have a valid "arg_tup" pointer for the cache entry
            let mut is_null = false;
            let datum = pg_sys::SysCacheGetAttr(
                pg_sys::SysCacheIdentifier_PROCOID as _,
                self.inner.as_ptr(),
                attribute as _,
                &mut is_null,
            );
            T::from_datum(datum, is_null)
        }
    }
}
