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
    BufferBarrier, BufferCopy, BufferImageCopy, ColorAttachment, CommandBuffer, DependencyInfo,
    DepthAttachment, ImageBarrier, ImageBlit, ImageCopy, MemoryBarrier, RenderingInfo,
    TimelinePoint,
};
pub use error::{Error, InvalidResource, Result, ShaderLanguage, UnsupportedOperation};
pub use presentation::{SurfaceCreateInfo, SurfaceFrame, Swapchain, SwapchainDesc};
pub use raw_window_handle;
pub use resource::{
    BindGroupDesc, BindGroupEntry, BindGroupLayoutDesc, BindGroupLayoutEntry, BindingResource,
    BlendComponent, BlendState, Buffer, BufferDesc, DepthBiasState, DepthState,
    GraphicsPipelineDesc, ImageDesc, ImageViewDesc, PipelineLayoutDesc, RasterState, SamplerDesc,
    ShaderDesc, ShaderSource, StencilFaceState, StencilState,
};
pub use types::{
    AddressMode, BindingType, BlendFactor, BlendOp, BufferUsages, Color, CompareOp, CullMode,
    Extent3d, FilterMode, FrontFace, ImageAspects, ImageState, ImageUsages, ImageViewType,
    IndexFormat, LoadOp, MemoryDomain, Origin3d, PipelineStages, PresentMode, PrimitiveTopology,
    QueueType, Rect, SampleCount, ShaderStage, ShaderStages, StencilOp, StoreOp, SurfaceStatus,
    TextureFormat, VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode, Viewport,
};

#[cfg(test)]
mod tests;
