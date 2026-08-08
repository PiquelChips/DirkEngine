#![allow(
    clippy::missing_errors_doc,
    reason = "backend methods share the crate Error contract; individual failure modes are backend-dependent"
)]

use std::sync::Arc;

use crate::{
    BindGroupDesc, BindGroupLayoutDesc, BufferDesc, CommandBuffer, Format, GraphicsPipelineDesc,
    ImageDesc, ImageViewDesc, PipelineLayoutDesc, QueueType, Result, SamplerDesc, ShaderDesc,
    SurfaceCreateInfo, SurfaceFrame, SwapchainDesc, TimelinePoint,
};

/// Application metadata and backend policy used during device creation.
pub struct RhiCreateInfo<'a> {
    /// Engine name exposed to graphics diagnostics.
    pub engine_name: &'a str,
    /// Engine semantic version.
    pub engine_version: (u32, u32, u32),
    /// Application name exposed to graphics diagnostics.
    pub application_name: &'a str,
    /// Application semantic version.
    pub application_version: (u32, u32, u32),
    /// Enables backend validation when available.
    pub validation: bool,
    /// Surface the selected device must be able to present to.
    ///
    /// Headless users may leave this unset and create a surface later, at
    /// which point presentation support can still be rejected by the backend.
    pub compatible_surface: Option<SurfaceCreateInfo>,
}

/// Selected device capabilities relevant to the renderer.
#[derive(Clone, Copy, Debug)]
pub struct Capabilities {
    /// Preferred depth attachment format.
    pub depth_format: Format,
    /// Maximum supported color/depth sample count.
    pub max_samples: crate::SampleCount,
    /// Maximum supported texture anisotropy.
    pub max_sampler_anisotropy: u16,
    /// Whether a distinct compute queue is available.
    pub dedicated_compute_queue: bool,
    /// Whether a distinct copy queue is available.
    pub dedicated_copy_queue: bool,
}

/// One queue submission, including presentation and timeline dependencies.
pub struct Submission<'a, B: Backend> {
    /// Recorded command buffers.
    pub command_buffers: &'a [B::CommandBuffer],
    /// Acquired frames waited on and signaled by this submission.
    pub surface_frames: &'a [&'a B::SurfaceFrame],
    /// Timeline values waited on before execution.
    pub wait_timelines: &'a [TimelinePoint<'a, B>],
    /// Timeline values signaled after execution.
    pub signal_timelines: &'a [TimelinePoint<'a, B>],
    /// Fence signaled after all submitted work completes.
    ///
    /// The backend may retain submitted resources through this fence, so it
    /// must not be reused until [`Rhi::wait_and_reset_fence`] succeeds.
    pub fence: &'a B::Fence,
}

/// Backend implementation contract for the RHI.
pub trait Backend: Sized + Send + Sync + 'static {
    /// Buffer resource.
    type Buffer: Clone + Send + Sync + 'static;
    /// Image resource, including externally-owned surface images.
    type Image: Clone + Send + Sync + 'static;
    /// Image view resource.
    type ImageView: Clone + Send + Sync + 'static;
    /// Texture sampler resource.
    type Sampler: Clone + Send + Sync + 'static;
    /// Shader module resource.
    type Shader: Clone + Send + Sync + 'static;
    /// Bind-group layout resource.
    type BindGroupLayout: Clone + Send + Sync + 'static;
    /// Bound resource group.
    type BindGroup: Clone + Send + Sync + 'static;
    /// Pipeline layout resource.
    type PipelineLayout: Clone + Send + Sync + 'static;
    /// Graphics pipeline resource.
    type GraphicsPipeline: Clone + Send + Sync + 'static;
    /// Command allocation pool.
    type CommandPool;
    /// Recorded command buffer.
    type CommandBuffer: CommandBuffer<Self>;
    /// Submission completion fence.
    type Fence;
    /// Timeline synchronization primitive.
    type TimelineSemaphore: Clone + Send + Sync + 'static;
    /// Presentation surface.
    type Surface: Clone + Send + Sync + 'static;
    /// Presentation swapchain.
    type Swapchain;
    /// Acquired presentation frame.
    type SurfaceFrame: SurfaceFrame<Self>;

    /// Creates a backend and selects its physical device.
    fn new(info: &RhiCreateInfo<'_>) -> Result<Self>;
    /// Returns selected device capabilities.
    fn capabilities(&self) -> Capabilities;
    /// Waits until all submitted device work completes.
    fn wait_idle(&self) -> Result<()>;
    /// Reclaims resources whose GPU use has completed.
    fn collect_garbage(&self) -> Result<()>;

    /// Creates a buffer.
    fn create_buffer(&self, desc: &BufferDesc<'_>) -> Result<Self::Buffer>;
    /// Writes bytes into host-visible buffer memory.
    fn write_buffer(&self, buffer: &Self::Buffer, offset: u64, data: &[u8]) -> Result<()>;
    /// Creates an image.
    fn create_image(&self, desc: &ImageDesc<'_>) -> Result<Self::Image>;
    /// Creates a view of an image.
    fn create_image_view(&self, desc: &ImageViewDesc<'_, Self>) -> Result<Self::ImageView>;
    /// Creates a sampler.
    fn create_sampler(&self, desc: &SamplerDesc<'_>) -> Result<Self::Sampler>;
    /// Creates a shader module.
    fn create_shader(&self, desc: &ShaderDesc<'_>) -> Result<Self::Shader>;
    /// Creates a bind-group layout.
    fn create_bind_group_layout(
        &self,
        desc: &BindGroupLayoutDesc<'_>,
    ) -> Result<Self::BindGroupLayout>;
    /// Creates a bound resource group.
    fn create_bind_group(&self, desc: &BindGroupDesc<'_, Self>) -> Result<Self::BindGroup>;
    /// Creates a pipeline layout.
    fn create_pipeline_layout(
        &self,
        desc: &PipelineLayoutDesc<'_, Self>,
    ) -> Result<Self::PipelineLayout>;
    /// Creates a graphics pipeline.
    fn create_graphics_pipeline(
        &self,
        desc: &GraphicsPipelineDesc<'_, Self>,
    ) -> Result<Self::GraphicsPipeline>;

    /// Creates a command pool for a semantic queue.
    fn create_command_pool(&self, queue: QueueType) -> Result<Self::CommandPool>;
    /// Allocates a command buffer from a pool.
    fn create_command_buffer(&self, pool: &Self::CommandPool) -> Result<Self::CommandBuffer>;
    /// Creates a fence.
    fn create_fence(&self, signaled: bool) -> Result<Self::Fence>;
    /// Waits for a fence and resets it for reuse.
    fn wait_and_reset_fence(&self, fence: &Self::Fence, timeout_ns: u64) -> Result<()>;
    /// Creates a timeline semaphore.
    fn create_timeline_semaphore(&self, initial_value: u64) -> Result<Self::TimelineSemaphore>;
    /// Waits for a timeline semaphore value.
    fn wait_timeline(
        &self,
        semaphore: &Self::TimelineSemaphore,
        value: u64,
        timeout_ns: u64,
    ) -> Result<()>;
    /// Returns a timeline semaphore's current value.
    fn timeline_value(&self, semaphore: &Self::TimelineSemaphore) -> Result<u64>;
    /// Submits command buffers and synchronization to a queue.
    fn submit(&self, queue: QueueType, submission: &Submission<'_, Self>) -> Result<()>;

    /// Creates a presentation surface.
    fn create_surface(&self, info: SurfaceCreateInfo) -> Result<Self::Surface>;
    /// Creates a presentation swapchain.
    fn create_swapchain(&self, desc: &SwapchainDesc<'_, Self>) -> Result<Self::Swapchain>;
    /// Acquires one presentation frame.
    fn acquire_frame(&self, swapchain: &mut Self::Swapchain) -> Result<Self::SurfaceFrame>;
    /// Recreates a presentation swapchain for a new extent.
    fn resize_swapchain(
        &self,
        swapchain: &mut Self::Swapchain,
        width: u32,
        height: u32,
    ) -> Result<()>;
    /// Presents one acquired frame.
    fn present(&self, frame: Self::SurfaceFrame) -> Result<()>;
}

/// Backend-neutral graphics device façade.
pub struct Rhi<B: Backend> {
    backend: Arc<B>,
}

impl<B: Backend> Clone for Rhi<B> {
    fn clone(&self) -> Self {
        Self {
            backend: self.backend.clone(),
        }
    }
}

impl<B: Backend> Rhi<B> {
    /// Creates an RHI using backend `B`.
    pub fn new(info: &RhiCreateInfo<'_>) -> Result<Self> {
        Ok(Self {
            backend: Arc::new(B::new(info)?),
        })
    }

    /// Creates an RHI from an already initialized backend.
    #[must_use]
    pub fn from_backend(backend: B) -> Self {
        Self {
            backend: Arc::new(backend),
        }
    }

    /// Returns the concrete backend for isolated native API integrations.
    ///
    /// Renderer code should normally use the neutral methods on [`Rhi`].
    #[must_use]
    pub fn backend(&self) -> &B {
        &self.backend
    }

    /// Returns selected device capabilities.
    #[must_use]
    pub fn capabilities(&self) -> Capabilities {
        self.backend.capabilities()
    }

    /// Waits until all submitted work completes.
    pub fn wait_idle(&self) -> Result<()> {
        self.backend.wait_idle()
    }

    /// Reclaims resources whose GPU use has completed.
    pub fn collect_garbage(&self) -> Result<()> {
        self.backend.collect_garbage()
    }

    /// Creates a buffer.
    pub fn create_buffer(&self, desc: &BufferDesc<'_>) -> Result<B::Buffer> {
        self.backend.create_buffer(desc)
    }

    /// Writes bytes into a host-visible buffer.
    pub fn write_buffer(&self, buffer: &B::Buffer, offset: u64, data: &[u8]) -> Result<()> {
        self.backend.write_buffer(buffer, offset, data)
    }

    /// Creates an image.
    pub fn create_image(&self, desc: &ImageDesc<'_>) -> Result<B::Image> {
        self.backend.create_image(desc)
    }

    /// Creates a view of an image.
    pub fn create_image_view(&self, desc: &ImageViewDesc<'_, B>) -> Result<B::ImageView> {
        self.backend.create_image_view(desc)
    }

    /// Creates a sampler.
    pub fn create_sampler(&self, desc: &SamplerDesc<'_>) -> Result<B::Sampler> {
        self.backend.create_sampler(desc)
    }

    /// Creates a shader module.
    pub fn create_shader(&self, desc: &ShaderDesc<'_>) -> Result<B::Shader> {
        self.backend.create_shader(desc)
    }

    /// Creates a bind-group layout.
    pub fn create_bind_group_layout(
        &self,
        desc: &BindGroupLayoutDesc<'_>,
    ) -> Result<B::BindGroupLayout> {
        self.backend.create_bind_group_layout(desc)
    }

    /// Creates a bind group.
    pub fn create_bind_group(&self, desc: &BindGroupDesc<'_, B>) -> Result<B::BindGroup> {
        self.backend.create_bind_group(desc)
    }

    /// Creates a pipeline layout.
    pub fn create_pipeline_layout(
        &self,
        desc: &PipelineLayoutDesc<'_, B>,
    ) -> Result<B::PipelineLayout> {
        self.backend.create_pipeline_layout(desc)
    }

    /// Creates a graphics pipeline.
    pub fn create_graphics_pipeline(
        &self,
        desc: &GraphicsPipelineDesc<'_, B>,
    ) -> Result<B::GraphicsPipeline> {
        self.backend.create_graphics_pipeline(desc)
    }

    /// Creates a command pool.
    pub fn create_command_pool(&self, queue: QueueType) -> Result<B::CommandPool> {
        self.backend.create_command_pool(queue)
    }

    /// Allocates a command buffer.
    pub fn create_command_buffer(&self, pool: &B::CommandPool) -> Result<B::CommandBuffer> {
        self.backend.create_command_buffer(pool)
    }

    /// Creates a fence.
    pub fn create_fence(&self, signaled: bool) -> Result<B::Fence> {
        self.backend.create_fence(signaled)
    }

    /// Waits for and resets a fence.
    pub fn wait_and_reset_fence(&self, fence: &B::Fence, timeout_ns: u64) -> Result<()> {
        self.backend.wait_and_reset_fence(fence, timeout_ns)
    }

    /// Creates a timeline semaphore.
    pub fn create_timeline_semaphore(&self, initial_value: u64) -> Result<B::TimelineSemaphore> {
        self.backend.create_timeline_semaphore(initial_value)
    }

    /// Waits for a timeline semaphore value.
    pub fn wait_timeline(
        &self,
        semaphore: &B::TimelineSemaphore,
        value: u64,
        timeout_ns: u64,
    ) -> Result<()> {
        self.backend.wait_timeline(semaphore, value, timeout_ns)
    }

    /// Returns the current timeline value.
    pub fn timeline_value(&self, semaphore: &B::TimelineSemaphore) -> Result<u64> {
        self.backend.timeline_value(semaphore)
    }

    /// Submits work to a semantic queue.
    pub fn submit(&self, queue: QueueType, submission: &Submission<'_, B>) -> Result<()> {
        self.backend.submit(queue, submission)
    }

    /// Creates a presentation surface.
    pub fn create_surface(&self, info: SurfaceCreateInfo) -> Result<B::Surface> {
        self.backend.create_surface(info)
    }

    /// Creates a swapchain.
    pub fn create_swapchain(&self, desc: &SwapchainDesc<'_, B>) -> Result<B::Swapchain> {
        self.backend.create_swapchain(desc)
    }

    /// Acquires one swapchain frame.
    pub fn acquire_frame(&self, swapchain: &mut B::Swapchain) -> Result<B::SurfaceFrame> {
        self.backend.acquire_frame(swapchain)
    }

    /// Resizes a swapchain.
    pub fn resize_swapchain(
        &self,
        swapchain: &mut B::Swapchain,
        width: u32,
        height: u32,
    ) -> Result<()> {
        self.backend.resize_swapchain(swapchain, width, height)
    }

    /// Presents one acquired frame.
    pub fn present(&self, frame: B::SurfaceFrame) -> Result<()> {
        self.backend.present(frame)
    }
}
