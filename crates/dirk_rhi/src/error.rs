use thiserror::Error;

/// Result returned by backend-neutral RHI operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors common to all graphics backends.
#[derive(Debug, Error)]
pub enum Error {
    /// The requested operation is unsupported by the selected backend.
    #[error("unsupported RHI operation: {0}")]
    Unsupported(&'static str),
    /// No graphics device satisfies the requested capabilities.
    #[error("no suitable graphics device was found")]
    NoDevice,
    /// The graphics device was lost.
    #[error("the graphics device was lost")]
    DeviceLost,
    /// The presentation surface must be recreated.
    #[error("the presentation surface is out of date")]
    SurfaceOutOfDate,
    /// Resource creation failed because the request is invalid.
    #[error("invalid resource description: {0}")]
    InvalidResource(&'static str),
    /// A backend-specific operation failed.
    #[error("graphics backend error")]
    Backend(#[from] anyhow::Error),
}
