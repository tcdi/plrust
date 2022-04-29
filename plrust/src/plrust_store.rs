use crate::logging::PgxGuestWriter;
use wasi_common::pipe::WritePipe;
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder};

pub(crate) struct PlRustStore {
    pub(crate) wasi: WasiCtx,
    pub(crate) host: crate::interface::Host,
    pub(crate) guest_data: crate::guest::GuestData,
}

impl Default for PlRustStore {
    fn default() -> Self {
        Self {
            wasi: WasiCtxBuilder::new()
                .stdout(Box::new(WritePipe::new(PgxGuestWriter::<false>)))
                .stderr(Box::new(WritePipe::new(PgxGuestWriter::<true>)))
                .build(),
            guest_data: crate::guest::GuestData::default(),
            host: crate::interface::Host::default(),
        }
    }
}
