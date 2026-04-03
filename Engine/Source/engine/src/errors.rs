use thiserror::Error;

pub type InitResult<T> = std::result::Result<T, EngineInitError>;
#[derive(Debug, Error)]
pub enum EngineInitError {
    #[error("failed to init platform: {0}")]
    PlatformInit(#[from] platform::PlatformError),
    #[error("failed to init renderer: {0}")]
    RendererInit(#[from] renderer::RendererError),
}

pub type RenderResult<T> = std::result::Result<T, EngineRenderError>;
#[derive(Debug, Error)]
pub enum EngineRenderError {
    #[error("renderer error: {0}")]
    RendererError(#[from] renderer::RendererError),
}

pub type ShutdownResult<T> = std::result::Result<T, EngineShutdownError>;
#[derive(Debug, Error)]
pub enum EngineShutdownError {}
