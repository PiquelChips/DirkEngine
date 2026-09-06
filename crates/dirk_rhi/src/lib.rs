//! Backend-neutral access to graphics devices.
//!
//! The RHI sits below the renderer's render graph. It describes resources,
//! commands and submissions in terms shared by explicit graphics APIs while
//! leaving allocation, synchronization and presentation to each backend.
//!
//! This crate defines and tests the portable contract. Concrete backend and
//! renderer integration crates are intentionally layered on top of it.

mod backend;
mod command;
mod error;
mod flags;
mod presentation;
mod resource;
mod types;

pub use backend::{
    Backend, Capabilities, Fence, FormatCapabilities, RhiCreateInfo, Submission, TimelineSemaphore,
};
pub use command::{
    BufferBarrier, BufferCopy, BufferImageCopy, ColorAttachment, CommandBuffer, DependencyInfo,
    DepthAttachment, ImageBarrier, ImageBlit, ImageCopy, MemoryBarrier, RenderingInfo,
    TimelinePoint,
};
pub use error::{
    Error, InvalidResource, InvalidResourceKind, Result, ShaderLanguage, UnsupportedOperation,
};
pub use presentation::{SurfaceCreateInfo, SurfaceFrame, SurfaceTarget, Swapchain, SwapchainDesc};
pub use raw_window_handle;
pub use resource::{
    BindGroupDesc, BindGroupEntry, BindGroupLayoutDesc, BindGroupLayoutEntry, BindingResource,
    BlendComponent, BlendState, Buffer, BufferDesc, ColorTargetState, DepthBiasState, DepthState,
    GraphicsPipelineDesc, ImageDesc, ImageViewDesc, PipelineLayoutDesc, RasterState, SamplerDesc,
    ShaderDesc, ShaderSource, StencilFaceState, StencilState,
};
pub use types::{
    AccessTypes, AddressMode, BindingType, BlendFactor, BlendOp, BufferUsages, Color, ColorSpace,
    ColorWrites, CompareOp, CullMode, Extent3d, FilterMode, FrontFace, ImageAspects,
    ImageDimension, ImageState, ImageUsages, ImageViewType, IndexFormat, LoadOp, MemoryDomain,
    Origin3d, PipelineStages, PresentMode, PrimitiveTopology, QueueTransfer, QueueType, Rect,
    SampleCount, SampleCounts, ShaderStage, ShaderStages, StencilOp, StoreOp, SurfaceFormat,
    SurfaceStatus, TextureFormat, VertexAttribute, VertexBufferLayout, VertexFormat,
    VertexStepMode, Viewport,
};

#[cfg(test)]
mod tests;
