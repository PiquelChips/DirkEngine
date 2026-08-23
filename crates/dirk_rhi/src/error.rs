use thiserror::Error;

/// Result returned by backend-neutral RHI operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors common to all graphics backends.
#[derive(Debug, Error)]
pub enum Error {
    /// The requested operation is unsupported by the selected backend.
    #[error("unsupported RHI operation: {0}")]
    Unsupported(#[from] UnsupportedOperation),
    /// No graphics device satisfies the requested capabilities.
    #[error("no suitable graphics device was found")]
    NoDevice,
    /// The graphics device was lost.
    #[error("the graphics device was lost")]
    DeviceLost,
    /// A timed wait on a synchronization primitive expired before the
    /// primitive reached its waited-for state.
    #[error("the synchronization wait timed out")]
    Timeout,
    /// The presentation surface must be recreated.
    #[error("the presentation surface is out of date")]
    SurfaceOutOfDate,
    /// Resource creation or use failed because the request is invalid.
    #[error("invalid resource description: {0}")]
    InvalidResource(#[from] InvalidResource),
    /// A backend-specific operation failed.
    ///
    /// The payload retains the native error chain, so `Display` reports the
    /// most specific context recorded by the backend.
    #[error("graphics backend error: {0}")]
    Backend(#[from] anyhow::Error),
}

/// Operations a backend may classify as unsupported.
///
/// This enumeration covers only the portable operations the RHI itself
/// defines; failures outside that vocabulary belong in [`Error::Backend`].
/// New members require extending this crate so every backend documents the
/// same gap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Error)]
#[non_exhaustive]
pub enum UnsupportedOperation {
    /// Filtered scaling between images is unavailable.
    #[error("image blits are not supported by this backend")]
    ImageBlit,
    /// The supplied shader source representation is not accepted natively.
    #[error("{0} shader source is not supported by this backend")]
    ShaderSource(#[source] ShaderLanguage),
}

/// Source representation of a shader program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Error)]
pub enum ShaderLanguage {
    /// SPIR-V words.
    #[error("SPIR-V")]
    SpirV,
    /// Metal Shading Language source text.
    #[error("Metal Shading Language")]
    Msl,
}

/// Reasons a resource creation or use request may be invalid.
///
/// Members describe the failure class so callers can match programmatically;
/// human-readable detail belongs in backend logs emitted where the failure is
/// detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Error)]
#[non_exhaustive]
pub enum InvalidResource {
    /// The resource belongs to another instance of this backend.
    #[error("resource belongs to a different RHI instance")]
    ForeignInstance,
    /// A requested size, offset, count, or range exceeds supported bounds.
    #[error("request exceeds a supported size, offset, or count limit")]
    OutOfRange,
    /// A required dimension, layer, level, or member count is zero.
    #[error("required dimension or count is zero")]
    Empty,
    /// The resource is not in a state permitting this operation.
    #[error("resource is not in a state permitting this operation")]
    BadState,
    /// The resource does not match its layout, format, or declared use.
    #[error("resource does not match its layout, format, or declared use")]
    Mismatch,
    /// The resource memory domain does not permit this host access.
    #[error("memory domain does not permit this host access")]
    NotHostAccessible,
    /// Input contains data the backend cannot represent natively.
    #[error("input contains data that cannot be represented")]
    Malformed,
}
