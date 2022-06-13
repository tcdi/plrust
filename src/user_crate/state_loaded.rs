use crate::user_crate::CrateState;
use libloading::os::unix::{Library, Symbol};
use pgx::pg_sys;
use std::path::Path;

impl CrateState for StateLoaded {}

#[must_use]
pub(crate) struct StateLoaded {
    #[allow(dead_code)] // Mostly for debugging
    fn_oid: pg_sys::Oid,
    #[allow(dead_code)] // We must hold this handle for `symbol`
    library: Library,
    symbol: Symbol<unsafe extern "C" fn(pg_sys::FunctionCallInfo) -> pg_sys::Datum>,
}

impl StateLoaded {
    #[tracing::instrument(level = "debug")]
    pub(crate) unsafe fn load(fn_oid: pg_sys::Oid, shared_object: &Path) -> eyre::Result<Self> {
        tracing::trace!("Loading {shared_object}");
        let library = Library::new(shared_object)?;
        let symbol_name = crate::plrust::symbol_name(fn_oid);
        tracing::trace!("Getting symbol {symbol_name}");
        let symbol = library.get(symbol_name.as_bytes())?;

        Ok(Self {
            fn_oid,
            library,
            symbol,
        })
    }

    pub(crate) unsafe fn evaluate(&self, fcinfo: pg_sys::FunctionCallInfo) -> pg_sys::Datum {
        (self.symbol)(fcinfo)
    }
}
