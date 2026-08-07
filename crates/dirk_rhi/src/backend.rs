//! Contract implemented by concrete graphics backends.

use crate::{
    BindGroupCreateInfo, BindGroupLayoutCreateInfo, BufferBarrier, BufferCopy, BufferCreateInfo,
    BufferImageCopy, CommandBufferBeginInfo, CommandBufferLevel, CommandPoolCreateInfo, Draw,
    DrawIndexed, Fence, Filter, GraphicsPipelineCreateInfo, ImageBarrier, ImageBlit, ImageCopy,
    ImageCreateInfo, ImageLayout, ImageViewCreateInfo, IndexFormat, PipelineLayoutCreateInfo,
    QueueType, Rect2D, RenderingInfo, Result, SamplerCreateInfo, SemaphoreKind,
    ShaderModuleCreateInfo, SubmitInfo, VertexBufferBinding, Viewport,
};

pub(crate) mod sealed {
    /// Prevents application crates from implementing the low-level backend contract.
    pub trait Sealed {}

    /// Prevents application crates from defining backend-specific interop adapters.
    pub trait InteropSealed {}
}

/// A complete graphics-device implementation.
///
/// This trait is sealed. Concrete Vulkan and Metal implementations live in
/// `dirk_rhi`; renderer code interacts with them through [`crate::Device`].
/// Backend-owned representations must remain safe to drop while submitted GPU
/// work references them, typically by retaining or deferring native deletion.
pub trait Backend: sealed::Sealed + Sized + 'static {
    /// Backend-owned buffer representation.
    type Buffer: 'static;
    /// Backend-owned image representation.
    type Image: 'static;
    /// Backend-owned image-view representation.
    type ImageView: 'static;
    /// Backend-owned sampler representation.
    type Sampler: 'static;
    /// Backend-owned shader-module representation.
    type ShaderModule: 'static;
    /// Backend-owned bind-group-layout representation.
    type BindGroupLayout: 'static;
    /// Backend-owned bind-group representation.
    type BindGroup: 'static;
    /// Backend-owned pipeline-layout representation.
    type PipelineLayout: 'static;
    /// Backend-owned graphics-pipeline representation.
    type Pipeline: 'static;
    /// Backend-owned command-pool representation.
    type CommandPool: 'static;
    /// Backend-owned command-buffer representation.
    type CommandBuffer: 'static;
    /// Backend-owned fence representation.
    type Fence: 'static;
    /// Backend-owned semaphore representation.
    type Semaphore: 'static;

    /// Waits for all submitted device work.
    fn wait_idle(&self) -> Result<()>;
    /// Flushes resources queued for deferred destruction.
    fn flush(&self);

    /// Creates a buffer.
    fn create_buffer(&self, info: &BufferCreateInfo<'_>) -> Result<Self::Buffer>;
    /// Writes host-visible buffer bytes.
    fn write_buffer(&self, buffer: &Self::Buffer, offset: u64, data: &[u8]) -> Result<()>;
    /// Creates an image.
    fn create_image(&self, info: &ImageCreateInfo<'_>) -> Result<Self::Image>;
    /// Creates an image view.
    fn create_image_view(
        &self,
        image: &Self::Image,
        info: &ImageViewCreateInfo<'_>,
    ) -> Result<Self::ImageView>;
    /// Creates a sampler.
    fn create_sampler(&self, info: &SamplerCreateInfo<'_>) -> Result<Self::Sampler>;
    /// Creates a shader module.
    fn create_shader_module(&self, info: &ShaderModuleCreateInfo<'_>)
    -> Result<Self::ShaderModule>;
    /// Creates a bind-group layout.
    fn create_bind_group_layout(
        &self,
        info: &BindGroupLayoutCreateInfo<'_>,
    ) -> Result<Self::BindGroupLayout>;
    /// Creates a bind group.
    fn create_bind_group(&self, info: &BindGroupCreateInfo<'_, Self>) -> Result<Self::BindGroup>;
    /// Creates a pipeline layout.
    fn create_pipeline_layout(
        &self,
        info: &PipelineLayoutCreateInfo<'_, Self>,
    ) -> Result<Self::PipelineLayout>;
    /// Creates a graphics pipeline.
    fn create_graphics_pipeline(
        &self,
        info: &GraphicsPipelineCreateInfo<'_, Self>,
    ) -> Result<Self::Pipeline>;

    /// Creates an explicit command pool.
    fn create_command_pool(&self, info: &CommandPoolCreateInfo<'_>) -> Result<Self::CommandPool>;
    /// Resets a command pool.
    fn reset_command_pool(&self, pool: &Self::CommandPool) -> Result<()>;
    /// Allocates a command buffer.
    fn allocate_command_buffer(
        &self,
        pool: &Self::CommandPool,
        level: CommandBufferLevel,
    ) -> Result<Self::CommandBuffer>;
    /// Begins command recording.
    fn begin_command_buffer(
        &self,
        command_buffer: &mut Self::CommandBuffer,
        info: &CommandBufferBeginInfo,
    ) -> Result<()>;
    /// Ends command recording.
    fn end_command_buffer(&self, command_buffer: &mut Self::CommandBuffer) -> Result<()>;
    /// Resets one command buffer.
    fn reset_command_buffer(&self, command_buffer: &mut Self::CommandBuffer) -> Result<()>;
    /// Records memory barriers.
    fn command_barriers(
        &self,
        command_buffer: &mut Self::CommandBuffer,
        image_barriers: &[ImageBarrier<'_, Self>],
        buffer_barriers: &[BufferBarrier<'_, Self>],
    ) -> Result<()>;
    /// Begins dynamic rendering.
    fn command_begin_rendering(
        &self,
        command_buffer: &mut Self::CommandBuffer,
        info: &RenderingInfo<'_, Self>,
    ) -> Result<()>;
    /// Ends dynamic rendering.
    fn command_end_rendering(&self, command_buffer: &mut Self::CommandBuffer) -> Result<()>;
    /// Records a buffer copy.
    fn command_copy_buffer(
        &self,
        command_buffer: &mut Self::CommandBuffer,
        source: &Self::Buffer,
        destination: &Self::Buffer,
        regions: &[BufferCopy],
    ) -> Result<()>;
    /// Records a buffer-to-image copy.
    fn command_copy_buffer_to_image(
        &self,
        command_buffer: &mut Self::CommandBuffer,
        source: &Self::Buffer,
        destination: &Self::Image,
        layout: ImageLayout,
        regions: &[BufferImageCopy],
    ) -> Result<()>;
    /// Records an image copy.
    #[allow(clippy::too_many_arguments)]
    fn command_copy_image(
        &self,
        command_buffer: &mut Self::CommandBuffer,
        source: &Self::Image,
        source_layout: ImageLayout,
        destination: &Self::Image,
        destination_layout: ImageLayout,
        regions: &[ImageCopy],
    ) -> Result<()>;
    /// Records a filtered image blit.
    #[allow(clippy::too_many_arguments)]
    fn command_blit_image(
        &self,
        command_buffer: &mut Self::CommandBuffer,
        source: &Self::Image,
        source_layout: ImageLayout,
        destination: &Self::Image,
        destination_layout: ImageLayout,
        regions: &[ImageBlit],
        filter: Filter,
    ) -> Result<()>;
    /// Sets a dynamic viewport.
    fn command_set_viewport(
        &self,
        command_buffer: &mut Self::CommandBuffer,
        viewport: Viewport,
    ) -> Result<()>;
    /// Sets a dynamic scissor rectangle.
    fn command_set_scissor(
        &self,
        command_buffer: &mut Self::CommandBuffer,
        scissor: Rect2D,
    ) -> Result<()>;
    /// Binds a graphics pipeline.
    fn command_bind_graphics_pipeline(
        &self,
        command_buffer: &mut Self::CommandBuffer,
        pipeline: &Self::Pipeline,
    ) -> Result<()>;
    /// Binds graphics resource groups.
    fn command_bind_graphics_groups(
        &self,
        command_buffer: &mut Self::CommandBuffer,
        layout: &Self::PipelineLayout,
        first_group: u32,
        groups: &[&crate::BindGroup<Self>],
        dynamic_offsets: &[u32],
    ) -> Result<()>;
    /// Binds vertex buffers.
    fn command_bind_vertex_buffers(
        &self,
        command_buffer: &mut Self::CommandBuffer,
        first_binding: u32,
        buffers: &[VertexBufferBinding<'_, Self>],
    ) -> Result<()>;
    /// Binds an index buffer.
    fn command_bind_index_buffer(
        &self,
        command_buffer: &mut Self::CommandBuffer,
        buffer: &Self::Buffer,
        offset: u64,
        format: IndexFormat,
    ) -> Result<()>;
    /// Records an indexed draw.
    fn command_draw_indexed(
        &self,
        command_buffer: &mut Self::CommandBuffer,
        draw: DrawIndexed,
    ) -> Result<()>;
    /// Records a non-indexed draw.
    fn command_draw(&self, command_buffer: &mut Self::CommandBuffer, draw: Draw) -> Result<()>;

    /// Creates a fence.
    fn create_fence(&self, signaled: bool) -> Result<Self::Fence>;
    /// Waits for a fence.
    fn wait_for_fence(&self, fence: &Self::Fence, timeout_ns: u64) -> Result<()>;
    /// Resets a fence.
    fn reset_fence(&self, fence: &mut Self::Fence) -> Result<()>;
    /// Creates a semaphore.
    fn create_semaphore(&self, kind: SemaphoreKind) -> Result<Self::Semaphore>;
    /// Waits for a timeline semaphore.
    fn wait_for_semaphore(
        &self,
        semaphore: &Self::Semaphore,
        value: u64,
        timeout_ns: u64,
    ) -> Result<()>;
    /// Returns a timeline semaphore value.
    fn semaphore_value(&self, semaphore: &Self::Semaphore) -> Result<u64>;
    /// Submits command buffers to a queue using the required completion fence.
    fn submit(
        &self,
        queue: QueueType,
        info: &SubmitInfo<'_, Self>,
        fence: &Fence<Self>,
    ) -> Result<()>;

    /// Backend-owned surface target supplied by a platform integration.
    #[cfg(feature = "presentation")]
    type SurfaceTarget: ?Sized;
    /// Backend-owned display surface.
    #[cfg(feature = "presentation")]
    type Surface: 'static;
    /// Backend-owned swapchain.
    #[cfg(feature = "presentation")]
    type Swapchain: 'static;
    /// Backend-owned acquired image token.
    #[cfg(feature = "presentation")]
    type RenderImage: 'static;

    /// Creates a display surface.
    #[cfg(feature = "presentation")]
    fn create_surface(&self, target: &Self::SurfaceTarget) -> Result<Self::Surface>;
    /// Creates a swapchain.
    #[cfg(feature = "presentation")]
    fn create_swapchain(
        &self,
        surface: &Self::Surface,
        info: &crate::SwapchainCreateInfo<'_>,
    ) -> Result<Self::Swapchain>;
    /// Recreates a swapchain in place.
    #[cfg(feature = "presentation")]
    fn recreate_swapchain(
        &self,
        swapchain: &mut Self::Swapchain,
        surface: &Self::Surface,
        info: &crate::SwapchainCreateInfo<'_>,
    ) -> Result<()>;
    /// Returns the swapchain's selected extent.
    #[cfg(feature = "presentation")]
    fn swapchain_extent(swapchain: &Self::Swapchain) -> crate::Extent2D;
    /// Returns the swapchain's selected image format.
    #[cfg(feature = "presentation")]
    fn swapchain_format(swapchain: &Self::Swapchain) -> crate::Format;
    /// Acquires a swapchain image.
    #[cfg(feature = "presentation")]
    fn acquire_render_image(
        &self,
        swapchain: &mut Self::Swapchain,
        timeout_ns: u64,
        signal: &Self::Semaphore,
    ) -> Result<Self::RenderImage>;
    /// Returns the image, view, and index represented by an acquired image.
    #[cfg(feature = "presentation")]
    fn render_image_parts(image: &Self::RenderImage) -> (&Self::Image, &Self::ImageView, u32);
    /// Presents an acquired image.
    #[cfg(feature = "presentation")]
    fn present(
        &self,
        swapchain: &mut Self::Swapchain,
        image: Self::RenderImage,
        waits: &[&Self::Semaphore],
    ) -> Result<()>;
    /// Releases an acquired image that will not be presented.
    #[cfg(feature = "presentation")]
    fn abandon_render_image(
        &self,
        swapchain: &mut Self::Swapchain,
        image: Self::RenderImage,
    ) -> Result<()>;
}

/// Sealed extension point for narrow, backend-specific compatibility adapters.
///
/// Concrete backend modules implement this trait for [`crate::Device`] and
/// return an adapter exposing only the native handles required by integrations
/// such as `egui-ash-renderer`. Generic RHI APIs remain backend-neutral.
pub trait BackendInterop: sealed::InteropSealed {
    /// Borrowed backend-specific adapter.
    type Adapter<'a>
    where
        Self: 'a;

    /// Returns the backend-specific compatibility adapter.
    fn interop(&self) -> Self::Adapter<'_>;
}
