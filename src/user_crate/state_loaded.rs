use crate::user_crate::CrateState;
use libloading::os::unix::{Library, Symbol};
use pgx::pg_sys;
use std::path::{Path, PathBuf};

impl CrateState for StateLoaded {}

#[must_use]
pub(crate) struct StateLoaded {
    #[allow(dead_code)] // Mostly for debugging
    fn_oid: pg_sys::Oid,
    #[allow(dead_code)] // Mostly for debugging
    symbol_name: String,
    #[allow(dead_code)] // We must hold this handle for `symbol`
    library: Library,
    shared_object: PathBuf,
    symbol: Symbol<unsafe extern "C" fn(pg_sys::FunctionCallInfo) -> pg_sys::Datum>,
}

impl StateLoaded {
    #[tracing::instrument(level = "debug")]
    pub(crate) unsafe fn load(fn_oid: pg_sys::Oid, shared_object: PathBuf) -> eyre::Result<Self> {
        tracing::trace!("Loading {shared_object}", shared_object = shared_object.display());
        let library = Library::new(&shared_object)?;
        let crate_name = crate::plrust::crate_name(fn_oid);

        #[cfg(any(all(target_os = "macos", target_arch = "x86_64"), feature = "force_enable_x86_64_darwin_generations"))]
        let crate_name = {
            let mut crate_name = crate_name;
            let (latest, path) = crate::generation::latest_generation(&crate_name, true)
                .unwrap_or_default();
            
            tracing::info!(path = %path.display(), "Got generation {latest}");
    
            crate_name.push_str(&format!("_{}", latest));
            crate_name
        };
        let symbol_name = crate_name + "_wrapper";

        tracing::trace!("Getting symbol {symbol_name}");
        let symbol = library.get(symbol_name.as_bytes())?;

        Ok(Self {
            fn_oid,
            symbol_name,
            library,
            shared_object,
            symbol,
        })
    }

    pub(crate) unsafe fn evaluate(&self, fcinfo: pg_sys::FunctionCallInfo) -> pg_sys::Datum {
        (self.symbol)(fcinfo)
    }

    pub(crate) fn close(self) -> eyre::Result<()> {
        let Self { fn_oid: _, library, symbol: _, shared_object: _, symbol_name: _ } = self;
        library.close()?;
        Ok(())
    }

    pub(crate) fn symbol_name(&self) -> &str {
        &self.symbol_name
    }

    pub(crate) fn shared_object(&self) -> &Path {
        &self.shared_object
    }
}
