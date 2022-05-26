use crate::{
    user_crate::CrateState,
};
use pgx::pg_sys;
use std::path::Path;
use libloading::{Library, Symbol};

impl<'a> CrateState for StateLoaded<'a> {}

#[must_use]
pub struct StateLoaded<'a> {
    fn_oid: pg_sys::Oid,
    library: Library,
    symbol: once_cell::sync::OnceCell<Symbol<'a, unsafe extern "C" fn(pg_sys::FunctionCallInfo) -> pg_sys::Datum>>,
}

impl<'a> StateLoaded<'a> {
    #[tracing::instrument(level = "debug", skip_all)]
    pub(crate) unsafe fn load(
        fn_oid: pg_sys::Oid,
        shared_object: &Path,
    ) -> eyre::Result<Self> {
        let library = Library::new(shared_object)?;

        let this = Self {
            fn_oid,
            library,
            symbol: Default::default(),
        };
        
        Ok(this)
    }

    #[tracing::instrument(level = "debug", skip_all)]
    pub unsafe fn symbol(&'a self) -> eyre::Result<&'a Symbol<'a, unsafe extern "C" fn(pg_sys::FunctionCallInfo) -> pg_sys::Datum>> {
        self.symbol.get_or_try_init(|| {
            let symbol_name = crate::plrust::symbol_name(self.fn_oid);
            let symbol = self.library.get(symbol_name.as_bytes())?;  
            Ok(symbol)
        })
    }

}