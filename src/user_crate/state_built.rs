use crate::user_crate::CrateState;
use std::path::{Path, PathBuf};

#[must_use]
pub struct StateBuilt {
    shared_object: PathBuf,
}

impl CrateState for StateBuilt {}

impl StateBuilt {
    #[tracing::instrument(level = "debug", skip_all)]
    pub fn new(shared_object: PathBuf) -> Self {
        Self {
            shared_object,
        }
    }
    #[tracing::instrument(level = "debug", skip_all)]
    pub fn shared_object(&self) -> &Path {
        self.shared_object.as_path()
    }
}
