use thiserror::Error;

pub type InitResult<T> = std::result::Result<T, EngineInitError>;
#[derive(Debug, Error)]
pub enum EngineInitError {}

pub type TickResult<T> = std::result::Result<T, EngineTickError>;
#[derive(Debug, Error)]
pub enum EngineTickError {}

pub type RenderResult<T> = std::result::Result<T, EngineRenderError>;
#[derive(Debug, Error)]
pub enum EngineRenderError {}

pub type ShutdownResult<T> = std::result::Result<T, EngineShutdownError>;
#[derive(Debug, Error)]
pub enum EngineShutdownError {}
