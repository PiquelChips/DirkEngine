use thiserror::Error;

pub type Result<T> = std::result::Result<T, PlatformError>;

#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("Error in winit event loop: {0}")]
    EventLoopError(#[from] winit::error::EventLoopError),
}
