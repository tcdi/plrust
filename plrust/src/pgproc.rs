use pgx::{pg_sys, FromDatum, IntoDatum};
use std::ptr::NonNull;

/// Provides a safe wrapper around a Postgres "SysCache" entry from `pg_catalog.pg_proc`.
pub(crate) struct PgProc {
    inner: NonNull<pg_sys::HeapTupleData>,
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
    pub(crate) fn new(pg_proc_oid: pg_sys::Oid) -> Option<PgProc> {
        unsafe {
            // SAFETY:  SearchSysCache1 will give us a valid HeapTuple or it'll return null.
            // Either way, using NonNull::new()? will make the right decision for us
            let entry = pg_sys::SearchSysCache1(
                pg_sys::SysCacheIdentifier_PROCOID as _,
                pg_proc_oid.into_datum().unwrap(),
            );
            Some(PgProc {
                inner: NonNull::new(entry)?,
            })
        }
    }

    #[inline]
    pub(crate) fn xmin(&self) -> pg_sys::TransactionId {
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

    pub(crate) fn prolang(&self) -> pg_sys::Oid {
        // SAFETY:  `prolang` has a NOT NULL constraint
        self.get_attr(pg_sys::Anum_pg_proc_prolang).unwrap()
    }

    pub(crate) fn prosrc(&self) -> String {
        // SAFETY:  `prosrc` has a NOT NULL constraint
        self.get_attr(pg_sys::Anum_pg_proc_prosrc).unwrap()
    }

    pub(crate) fn proargnames(&self) -> Vec<Option<String>> {
        self.get_attr(pg_sys::Anum_pg_proc_proargnames)
            .unwrap_or_default()
    }

    pub(crate) fn proargtypes(&self) -> Vec<pg_sys::Oid> {
        self.get_attr(pg_sys::Anum_pg_proc_proargtypes)
            .unwrap_or_default()
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
