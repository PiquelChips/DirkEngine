//! Errors returned by the RHI contract.

use std::{error, fmt};

/// A result returned by an RHI operation.
pub type Result<T> = std::result::Result<T, Error>;

/// An error reported by descriptor validation or a concrete backend.
#[derive(Debug)]
pub enum Error {
    /// A descriptor contains an invalid combination of values.
    InvalidDescriptor {
        /// The kind of descriptor that failed validation.
        descriptor: &'static str,
        /// A concise explanation of the invalid value.
        reason: &'static str,
    },
    /// A resource belongs to a different device.
    DeviceMismatch {
        /// The operation that received the mismatched resource.
        operation: &'static str,
    },
    /// A command buffer operation is invalid in its current state.
    InvalidCommandBufferState {
        /// The state required by the operation.
        expected: &'static str,
        /// The command buffer's actual state.
        actual: &'static str,
    },
    /// A concrete graphics backend reported an error.
    Backend(Box<dyn error::Error + Send + Sync>),
}

impl Error {
    /// Wraps an error reported by a concrete backend.
    pub fn backend(error: impl error::Error + Send + Sync + 'static) -> Self {
        Self::Backend(Box::new(error))
    }

    pub(crate) const fn invalid_descriptor(descriptor: &'static str, reason: &'static str) -> Self {
        Self::InvalidDescriptor { descriptor, reason }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDescriptor { descriptor, reason } => {
                write!(formatter, "invalid {descriptor} descriptor: {reason}")
            }
            Self::DeviceMismatch { operation } => {
                write!(
                    formatter,
                    "{operation} received a resource from another device"
                )
            }
            Self::InvalidCommandBufferState { expected, actual } => write!(
                formatter,
                "command buffer must be {expected}, but is {actual}"
            ),
            Self::Backend(error) => error.fmt(formatter),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::Backend(error) => Some(error.as_ref()),
            Self::InvalidDescriptor { .. }
            | Self::DeviceMismatch { .. }
            | Self::InvalidCommandBufferState { .. } => None,
        }
    }
}
