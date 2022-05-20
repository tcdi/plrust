use std::path::{PathBuf, Path};
use std::process::Output;
use crate::user_crate::CrateState;

#[must_use]
pub struct StateBuilt {
    shared_object: PathBuf,
    output: Output,
}

impl CrateState for StateBuilt {}

impl StateBuilt {
    #[tracing::instrument(level = "debug", skip_all)]
    pub fn new(shared_object: PathBuf, output: Output) -> Self {
        Self {
            shared_object,
            output,
        }
    }
    #[tracing::instrument(level = "debug", skip_all)]
    pub fn shared_object(&self) -> &Path {
        self.shared_object.as_path()
    }
    #[tracing::instrument(level = "debug", skip_all)]
    pub fn output(&self) -> &Output {
        &self.output
    }
}