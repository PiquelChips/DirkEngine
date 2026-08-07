//! Backend-neutral GPU resources and command recording for `DirkEngine`.
//!
//! The renderer owns frame scheduling and its render graph. This crate owns
//! the lower-level contract implemented by concrete graphics backends.

#![allow(clippy::missing_errors_doc)]

mod backend;
mod command;
mod error;
mod flags;
#[cfg(feature = "presentation")]
mod presentation;
mod resource;
#[cfg(test)]
mod test_backend;
mod types;

pub use backend::{Backend, BackendInterop};
pub use command::*;
pub use error::{Error, Result};
#[cfg(feature = "presentation")]
pub use presentation::{PresentMode, RenderImage, Surface, Swapchain, SwapchainCreateInfo};
pub use resource::*;
pub use types::*;
