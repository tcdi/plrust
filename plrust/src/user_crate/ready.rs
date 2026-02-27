/*
Portions Copyright 2021-2025 Technology Concepts & Design, Inc.

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

use libloading::os::unix::{Library, Symbol};
use pgrx::pg_sys;

use crate::user_crate::CrateState;

impl CrateState for FnReady {}

/// Ready-to-evaluate PL/Rust function
///
/// - Requires: dlopened artifact
/// - Produces: evaluation of the PL/Rust function
#[must_use]
pub(crate) struct FnReady {
    generation_number: u64,
    symbol_name: String,
    #[allow(dead_code)] // We must hold this handle for `symbol`
    library: Library,
    symbol: Symbol<unsafe extern "C" fn(pg_sys::FunctionCallInfo) -> pg_sys::Datum>,

    // used to hang onto the thing where the "shared object bytes" were written
    // mainly, this is to hold the `Memfd` instance on Linux so that we can support
    // loading more than one user function .so at a time.  Linux seems to have a memory
    // of what it dlopen()'d based on the file descriptor number
    //
    // and it's different based on platform!
    #[cfg(target_os = "linux")]
    _file_holder: memfd::Memfd,
    #[cfg(not(target_os = "linux"))]
    _file_holder: (),
}

impl FnReady {
    #[tracing::instrument(level = "debug", skip_all, fields(db_oid = %db_oid, fn_oid = %fn_oid))]
    pub(crate) unsafe fn load(
        generation_number: u64,
        db_oid: pg_sys::Oid,
        fn_oid: pg_sys::Oid,
        symbol: Option<String>,
        shared_object: Vec<u8>,
    ) -> eyre::Result<Self> {
        #[cfg(target_os = "linux")]
        let (file_holder, library) = {
            // for Linux we write the `shared_object` bytes to an anonymous file of exactly the
            // right size.  Then we ask `libloading::Library` to "dlopen" it using a direct path
            // to its file descriptor in "/proc/self/fd/{raw_fd}".
            //
            // This is an added "safety" measure as we can (reasonably) assure ourselves that the
            // file won't be overwritten between when we finish writing it and when it is dlopen'd
            use std::io::Write;
            use std::os::unix::io::AsRawFd;

            let mfd = memfd::MemfdOptions::default()
                .allow_sealing(true)
                .create(&format!("plrust-fn-{db_oid}-{fn_oid}-{generation_number}"))?;

            // set the filesize to exactly what we know it should be
            mfd.as_file().set_len(shared_object.len() as u64)?;

            // make sure we can't change the filesize
            mfd.add_seals(&[memfd::FileSeal::SealShrink, memfd::FileSeal::SealGrow])?;
            mfd.add_seal(memfd::FileSeal::SealSeal)?;

            // and write the shared_object bytes
            mfd.as_file().write_all(&shared_object)?;

            // generate a direct filename to the underlying raw file descriptor that `mfd` created
            let raw_fd = mfd.as_raw_fd();
            let filename = format!("/proc/self/fd/{raw_fd}");

            // finally, load the library
            let library = unsafe { Library::new(&filename)? };

            // we need to also return the `Memfd` instance as well as if it gets dropped
            // Linux might re-use its filedescriptor and dlopen() won't open the new library
            // behind it
            (mfd, library)
        };

        #[cfg(not(target_os = "linux"))]
        let (file_holder, library) = {
            // for all other platforms we write the `shared_object` bytes out to a temporary file rooted in our
            // configured `plrust.work_dir`.  This will get removed from disk when this function
            // exists, which is fine because we'll have dlopen()'d it by then and no longer need it
            let temp_so_file = tempfile::Builder::new().tempfile_in(crate::gucs::work_dir())?;
            std::fs::write(&temp_so_file, shared_object)?;

            let library = unsafe { Library::new(temp_so_file.path())? };

            // just to be obvious, the temp_so_file gets deleted here.  Now that it's been loaded, we don't
            // need it.  If any of the above failed and returned an Error, it'll still get deleted when
            // the function returns.
            drop(temp_so_file);

            ((), library)
        };

        let symbol_name = symbol.unwrap_or_else(|| crate::plrust::symbol_name(db_oid, fn_oid));
        let symbol_name = symbol_name + "_wrapper"; // + "_wrapper" b/c pgrx' `#[pg_extern]` adds that

        tracing::trace!("Getting symbol `{symbol_name}`");
        let symbol = unsafe { library.get(symbol_name.as_bytes())? };

        Ok(Self {
            generation_number,
            symbol_name,
            library,
            symbol,
            _file_holder: file_holder,
        })
    }

    #[tracing::instrument(level = "debug", skip_all, fields(?fcinfo))]
    pub(crate) unsafe fn evaluate(&self, fcinfo: pg_sys::FunctionCallInfo) -> pg_sys::Datum {
        // SAFETY:  First off, `self.symbol` is some function in the dlopened shared library, so
        // FFI into that is inherently unsafe.  Secondly, it's an FFI function, so we need to protect
        // that boundary to properly handle Rust panics and Postgres errors, hence the use of
        // `pg_guard_ffi_boundary()`.
        unsafe { pg_sys::submodules::ffi::pg_guard_ffi_boundary(|| (self.symbol)(fcinfo)) }
    }

    #[tracing::instrument(
        level = "debug",
        skip_all,
        fields(
            symbol_name = %self.symbol_name,
        ))]
    pub(crate) fn close(self) -> eyre::Result<()> {
        let Self {
            generation_number: _,
            library,
            symbol: _,
            symbol_name: _,
            _file_holder: _,
        } = self;
        library.close()?;
        Ok(())
    }

    pub(crate) fn symbol_name(&self) -> &str {
        &self.symbol_name
    }

    #[inline]
    pub(crate) fn generation_number(&self) -> u64 {
        self.generation_number
    }
}
