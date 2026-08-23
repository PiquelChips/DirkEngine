#![allow(
    clippy::missing_errors_doc,
    reason = "backend methods share the crate Error contract; individual failure modes are backend-dependent"
)]

use std::fmt::Debug;

use raw_window_handle::{DisplayHandle, WindowHandle};

use crate::{
    BindGroupDesc, BindGroupLayoutDesc, Buffer, BufferDesc, CommandBuffer, GraphicsPipelineDesc,
    ImageDesc, ImageViewDesc, PipelineLayoutDesc, QueueType, Result, SamplerDesc, ShaderDesc,
    SurfaceCreateInfo, SurfaceFrame, Swapchain, SwapchainDesc, TextureFormat, TimelinePoint,
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
    /// Display and window handles the selected device must be able to present
    /// to.
    ///
    /// Headless users may leave this unset and create a surface later, at
    /// which point presentation support can still be rejected by the backend.
    pub compatible_surface: Option<(DisplayHandle<'a>, WindowHandle<'a>)>,
}

/// Selected device capabilities relevant to the renderer.
#[derive(Clone, Copy, Debug)]
pub struct Capabilities {
    /// Maximum supported color/depth sample count.
    pub max_samples: crate::SampleCount,
    /// Maximum supported texture anisotropy.
    pub max_sampler_anisotropy: u16,
    /// Minimum alignment for a uniform-buffer binding offset, in bytes.
    pub min_uniform_buffer_offset_alignment: u64,
    /// Minimum alignment for a storage-buffer binding offset, in bytes.
    pub min_storage_buffer_offset_alignment: u64,
    /// Whether a distinct compute queue is available.
    pub dedicated_compute_queue: bool,
    /// Whether a distinct copy queue is available.
    pub dedicated_copy_queue: bool,
}

/// CPU-waitable submission completion primitive.
pub trait Fence: Send + Sync + 'static {
    /// Waits until this fence signals or `timeout_ns` nanoseconds elapse.
    ///
    /// Returns `Ok(())` once the fence is signaled, including for a fence
    /// created signaled or one whose submission completed before the call.
    /// Returns [`Error::Timeout`](crate::Error::Timeout) when the timeout
    /// expires first; the fence may still signal afterwards.
    ///
    /// # Synchronization
    ///
    /// Native fence state is not internally synchronized: at most one thread
    /// may wait on this fence at a time, and callers must order waits
    /// against [`Self::reset`](Fence::reset) and against the submission that
    /// signals the fence.
    fn wait(&self, timeout_ns: u64) -> Result<()>;
    /// Resets this signaled fence for reuse.
    ///
    /// # Synchronization
    ///
    /// The caller must ensure no thread is concurrently waiting on or
    /// resetting this fence, and that every submission signaling it has been
    /// waited on (or otherwise completed) first; resetting a fence that is
    /// still pending on a submission or under concurrent state access is
    /// invalid in the native APIs.
    fn reset(&self) -> Result<()>;
}

/// Monotonically increasing GPU synchronization primitive.
pub trait TimelineSemaphore: Clone + Send + Sync + 'static {
    /// Waits until this semaphore reaches `value` or `timeout_ns`
    /// nanoseconds elapse.
    ///
    /// Returns `Ok(())` once the semaphore's payload is at least `value`,
    /// including when it already exceeds it. Returns
    /// [`Error::Timeout`](crate::Error::Timeout) when the timeout expires
    /// first; the semaphore may still reach `value` afterwards.
    ///
    /// # Synchronization
    ///
    /// Native semaphore state is not internally synchronized: at most one
    /// thread may wait on or observe this semaphore at a time, and callers
    /// must order waits and calls to [`Self::value`](TimelineSemaphore::value)
    /// against the submissions that signal it. Cloned handles alias the same
    /// native semaphore and do not add synchronization.
    fn wait(&self, value: u64, timeout_ns: u64) -> Result<()>;
    /// Returns this semaphore's current value.
    fn value(&self) -> Result<u64>;
}

/// One queue submission, including presentation and timeline dependencies.
///
/// # Frame presentation protocol
///
/// Frames acquired from a swapchain are coupled to rendering through
/// `surface_frames`. For each listed frame the backend:
///
/// 1. waits, before execution begins, on the semaphore produced by
///    [`Swapchain::acquire`] for that frame;
/// 2. signals, once all recorded work completes, the semaphore consumed by
///    [`Swapchain::present`] for that frame.
///
/// A typical frame therefore acquires a frame, records into its image and
/// view, lists it in `surface_frames`, submits, and finally presents it.
/// Each frame must be submitted exactly once between acquisition and
/// presentation; see [`Swapchain`] for the full lifecycle and pacing rules.
pub struct Submission<'a, B: Backend> {
    /// Recorded command buffers.
    pub command_buffers: &'a [&'a B::CommandBuffer],
    /// Acquired frames whose presentation dependencies are handled by this
    /// submission. See the type-level documentation for the protocol.
    pub surface_frames: &'a [&'a B::SurfaceFrame],
    /// Timeline values waited on before execution.
    pub wait_timelines: &'a [TimelinePoint<'a, B>],
    /// Timeline values signaled after execution.
    pub signal_timelines: &'a [TimelinePoint<'a, B>],
    /// Optional fence signaled after all submitted work completes.
    ///
    /// A timeline-only submission may leave this unset. When provided, the
    /// backend may retain submitted resources through this fence, so it must
    /// not be reused until [`Fence::wait`] succeeds.
    pub fence: Option<&'a B::Fence>,
}

/// Backend implementation contract for the RHI.
pub trait Backend: Sized + Send + Sync + 'static {
    /// Buffer resource.
    type Buffer: Buffer + Debug;
    /// Image resource, including externally-owned surface images.
    type Image: Clone + Debug + Send + Sync + 'static;
    /// Image view resource.
    type ImageView: Clone + Debug + Send + Sync + 'static;
    /// Texture sampler resource.
    type Sampler: Clone + Debug + Send + Sync + 'static;
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
    type CommandPool: Send + 'static;
    /// Recorded command buffer.
    type CommandBuffer: CommandBuffer<Self>;
    /// Submission completion fence.
    type Fence: Fence;
    /// Timeline synchronization primitive.
    type TimelineSemaphore: TimelineSemaphore;
    /// Presentation surface.
    type Surface: Clone + Send + Sync + 'static;
    /// Presentation swapchain.
    type Swapchain: Swapchain<Self>;
    /// Acquired presentation frame.
    type SurfaceFrame: SurfaceFrame<Self>;

    /// Creates a backend and selects its physical device.
    fn new(info: &RhiCreateInfo<'_>) -> Result<Self>;
    /// Returns selected device capabilities.
    fn capabilities(&self) -> Capabilities;
    /// Returns the depth attachment formats supported by the selected
    /// device, ordered from most to least preferred.
    fn supported_depth_formats(&self) -> &'static [TextureFormat];
    /// Waits until all submitted device work completes.
    fn wait_idle(&self) -> Result<()>;
    /// Reclaims resources whose GPU use has completed.
    fn collect_garbage(&self) -> Result<()>;

    /// Creates a buffer.
    fn create_buffer(&self, desc: &BufferDesc<'_>) -> Result<Self::Buffer>;
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
    fn create_command_buffer(&self, pool: &mut Self::CommandPool) -> Result<Self::CommandBuffer>;
    /// Creates a fence.
    fn create_fence(&self, signaled: bool) -> Result<Self::Fence>;
    /// Creates a timeline semaphore.
    fn create_timeline_semaphore(&self, initial_value: u64) -> Result<Self::TimelineSemaphore>;
    /// Submits command buffers and synchronization to a queue.
    fn submit(&self, queue: QueueType, submission: &Submission<'_, Self>) -> Result<()>;

    /// Creates a presentation surface.
    fn create_surface(&self, info: SurfaceCreateInfo<'_>) -> Result<Self::Surface>;
    /// Creates a presentation swapchain.
    fn create_swapchain(&self, desc: &SwapchainDesc<'_, Self>) -> Result<Self::Swapchain>;
}
