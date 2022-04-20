use crate::guest_with_oids::GuestWithOids;
use pgx::pg_sys;
use std::collections::BTreeMap;
use wasmtime::Engine;

pub(crate) struct WasmExecutor {
    engine: Engine,
    guests: BTreeMap<pg_sys::Oid, GuestWithOids>,
}

impl WasmExecutor {
    pub(crate) fn new() -> eyre::Result<Self> {
        let engine = Engine::default();

        Ok(Self {
            engine,
            guests: Default::default(),
        })
    }

    pub(crate) fn engine(&self) -> &Engine {
        &self.engine
    }

    pub(crate) fn instantiate(&mut self, fn_oid: pg_sys::Oid) -> eyre::Result<&mut GuestWithOids> {
        let guest = GuestWithOids::new(self, fn_oid)?;
        return Ok(self.guests.entry(fn_oid).or_insert(guest));
    }

    pub(crate) fn remove(&mut self, fn_oid: &pg_sys::Oid) -> Option<GuestWithOids> {
        self.guests.remove(fn_oid)
    }

    pub(crate) fn guest(&mut self, fn_oid: &pg_sys::Oid) -> Option<&mut GuestWithOids> {
        self.guests.get_mut(fn_oid)
    }
}
