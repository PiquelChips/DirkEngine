//! This module contains all the enigne building related stuff.

use crate::{Engine, errors::EngineBuildError};

/// Engine builder struct. A utility to configure the engine how you want.
pub struct EngineBuilder {}

impl EngineBuilder {
    /// Actually builds the engine from the configuration specified by the user.
    pub fn build(self) -> Result<Engine, EngineBuildError> {
        Engine::build(self)
    }
}
