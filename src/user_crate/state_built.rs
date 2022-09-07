use crate::user_crate::{CrateState, StateLoaded};
use pgx::pg_sys;
use std::path::{Path, PathBuf};

#[must_use]
pub(crate) struct StateBuilt {
    db_oid: pg_sys::Oid,
    fn_oid: pg_sys::Oid,
    shared_object: PathBuf,
}

impl CrateState for StateBuilt {}

impl StateBuilt {
    #[tracing::instrument(level = "debug", skip_all)]
    pub(crate) fn new(db_oid: pg_sys::Oid, fn_oid: pg_sys::Oid, shared_object: PathBuf) -> Self {
        Self {
            db_oid,
            fn_oid,
            shared_object,
        }
    }

    pub(crate) fn shared_object(&self) -> &Path {
        &self.shared_object
    }

    pub(crate) fn fn_oid(&self) -> &u32 {
        &self.fn_oid
    }

    pub(crate) fn db_oid(&self) -> &u32 {
        &self.db_oid
    }

    #[tracing::instrument(level = "debug", skip_all, fields(db_oid = %self.db_oid, fn_oid = %self.fn_oid, shared_object = %self.shared_object.display()))]
    pub(crate) unsafe fn load(self) -> eyre::Result<StateLoaded> {
        StateLoaded::load(self.db_oid, self.fn_oid, self.shared_object)
    }
}
