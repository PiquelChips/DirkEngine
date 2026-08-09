//! Backend-neutral access to graphics devices.
//!
//! The RHI sits below the renderer's render graph. It describes resources,
//! commands and submissions in terms shared by explicit graphics APIs while
//! leaving allocation, synchronization and presentation to each backend.

mod backend;
mod command;
mod error;
mod flags;
mod presentation;
mod resource;
mod types;

pub use backend::{Backend, Capabilities, Fence, RhiCreateInfo, Submission, TimelineSemaphore};
pub use command::{
    BufferCopy, BufferImageCopy, ColorAttachment, CommandBuffer, DependencyInfo, DepthAttachment,
    ImageBarrier, ImageBlit, ImageCopy, RenderingInfo, TimelinePoint,
};
pub use error::{Error, Result};
pub use presentation::{SurfaceCreateInfo, SurfaceFrame, Swapchain, SwapchainDesc};
pub use resource::{
    BindGroupDesc, BindGroupEntry, BindGroupLayoutDesc, BindGroupLayoutEntry, BindingResource,
    Buffer, BufferDesc, GraphicsPipelineDesc, ImageDesc, ImageViewDesc, PipelineLayoutDesc,
    SamplerDesc, ShaderDesc, ShaderSource,
};
pub use types::*;

#[cfg(test)]
mod tests;
