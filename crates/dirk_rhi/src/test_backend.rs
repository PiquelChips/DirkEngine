#[cfg(feature = "presentation")]
use std::cell::Cell;

use crate::*;

#[derive(Default)]
pub(crate) struct FakeBackend {
    #[cfg(feature = "presentation")]
    abandoned_images: Cell<usize>,
    #[cfg(feature = "presentation")]
    presented_images: Cell<usize>,
}

impl backend::sealed::Sealed for FakeBackend {}

pub(crate) fn device() -> Device<FakeBackend> {
    Device::new(FakeBackend::default())
}

impl Backend for FakeBackend {
    type Buffer = ();
    type Image = ();
    type ImageView = ();
    type Sampler = ();
    type ShaderModule = ();
    type BindGroupLayout = ();
    type BindGroup = ();
    type PipelineLayout = ();
    type Pipeline = ();
    type CommandPool = ();
    type CommandBuffer = ();
    type Fence = ();
    type Semaphore = ();

    fn wait_idle(&self) -> Result<()> {
        Ok(())
    }

    fn flush(&self) {}

    fn create_buffer(&self, _info: &BufferCreateInfo<'_>) -> Result<Self::Buffer> {
        Ok(())
    }

    fn write_buffer(&self, _buffer: &Self::Buffer, _offset: u64, _data: &[u8]) -> Result<()> {
        Ok(())
    }

    fn create_image(&self, _info: &ImageCreateInfo<'_>) -> Result<Self::Image> {
        Ok(())
    }

    fn create_image_view(
        &self,
        _image: &Self::Image,
        _info: &ImageViewCreateInfo<'_>,
    ) -> Result<Self::ImageView> {
        Ok(())
    }

    fn create_sampler(&self, _info: &SamplerCreateInfo<'_>) -> Result<Self::Sampler> {
        Ok(())
    }

    fn create_shader_module(
        &self,
        _info: &ShaderModuleCreateInfo<'_>,
    ) -> Result<Self::ShaderModule> {
        Ok(())
    }

    fn create_bind_group_layout(
        &self,
        _info: &BindGroupLayoutCreateInfo<'_>,
    ) -> Result<Self::BindGroupLayout> {
        Ok(())
    }

    fn create_bind_group(&self, _info: &BindGroupCreateInfo<'_, Self>) -> Result<Self::BindGroup> {
        Ok(())
    }

    fn create_pipeline_layout(
        &self,
        _info: &PipelineLayoutCreateInfo<'_, Self>,
    ) -> Result<Self::PipelineLayout> {
        Ok(())
    }

    fn create_graphics_pipeline(
        &self,
        _info: &GraphicsPipelineCreateInfo<'_, Self>,
    ) -> Result<Self::Pipeline> {
        Ok(())
    }

    fn create_command_pool(&self, _info: &CommandPoolCreateInfo<'_>) -> Result<Self::CommandPool> {
        Ok(())
    }

    fn reset_command_pool(&self, _pool: &Self::CommandPool) -> Result<()> {
        Ok(())
    }

    fn allocate_command_buffer(
        &self,
        _pool: &Self::CommandPool,
        _level: CommandBufferLevel,
    ) -> Result<Self::CommandBuffer> {
        Ok(())
    }

    fn begin_command_buffer(
        &self,
        _command_buffer: &mut Self::CommandBuffer,
        _info: &CommandBufferBeginInfo,
    ) -> Result<()> {
        Ok(())
    }

    fn end_command_buffer(&self, _command_buffer: &mut Self::CommandBuffer) -> Result<()> {
        Ok(())
    }

    fn reset_command_buffer(&self, _command_buffer: &mut Self::CommandBuffer) -> Result<()> {
        Ok(())
    }

    fn command_barriers(
        &self,
        _command_buffer: &mut Self::CommandBuffer,
        _image_barriers: &[ImageBarrier<'_, Self>],
        _buffer_barriers: &[BufferBarrier<'_, Self>],
    ) -> Result<()> {
        Ok(())
    }

    fn command_begin_rendering(
        &self,
        _command_buffer: &mut Self::CommandBuffer,
        _info: &RenderingInfo<'_, Self>,
    ) -> Result<()> {
        Ok(())
    }

    fn command_end_rendering(&self, _command_buffer: &mut Self::CommandBuffer) -> Result<()> {
        Ok(())
    }

    fn command_copy_buffer(
        &self,
        _command_buffer: &mut Self::CommandBuffer,
        _source: &Self::Buffer,
        _destination: &Self::Buffer,
        _regions: &[BufferCopy],
    ) -> Result<()> {
        Ok(())
    }

    fn command_copy_buffer_to_image(
        &self,
        _command_buffer: &mut Self::CommandBuffer,
        _source: &Self::Buffer,
        _destination: &Self::Image,
        _layout: ImageLayout,
        _regions: &[BufferImageCopy],
    ) -> Result<()> {
        Ok(())
    }

    fn command_copy_image(
        &self,
        _command_buffer: &mut Self::CommandBuffer,
        _source: &Self::Image,
        _source_layout: ImageLayout,
        _destination: &Self::Image,
        _destination_layout: ImageLayout,
        _regions: &[ImageCopy],
    ) -> Result<()> {
        Ok(())
    }

    fn command_blit_image(
        &self,
        _command_buffer: &mut Self::CommandBuffer,
        _source: &Self::Image,
        _source_layout: ImageLayout,
        _destination: &Self::Image,
        _destination_layout: ImageLayout,
        _regions: &[ImageBlit],
        _filter: Filter,
    ) -> Result<()> {
        Ok(())
    }

    fn command_set_viewport(
        &self,
        _command_buffer: &mut Self::CommandBuffer,
        _viewport: Viewport,
    ) -> Result<()> {
        Ok(())
    }

    fn command_set_scissor(
        &self,
        _command_buffer: &mut Self::CommandBuffer,
        _scissor: Rect2D,
    ) -> Result<()> {
        Ok(())
    }

    fn command_bind_graphics_pipeline(
        &self,
        _command_buffer: &mut Self::CommandBuffer,
        _pipeline: &Self::Pipeline,
    ) -> Result<()> {
        Ok(())
    }

    fn command_bind_graphics_groups(
        &self,
        _command_buffer: &mut Self::CommandBuffer,
        _layout: &Self::PipelineLayout,
        _first_group: u32,
        _groups: &[&BindGroup<Self>],
        _dynamic_offsets: &[u32],
    ) -> Result<()> {
        Ok(())
    }

    fn command_bind_vertex_buffers(
        &self,
        _command_buffer: &mut Self::CommandBuffer,
        _first_binding: u32,
        _buffers: &[VertexBufferBinding<'_, Self>],
    ) -> Result<()> {
        Ok(())
    }

    fn command_bind_index_buffer(
        &self,
        _command_buffer: &mut Self::CommandBuffer,
        _buffer: &Self::Buffer,
        _offset: u64,
        _format: IndexFormat,
    ) -> Result<()> {
        Ok(())
    }

    fn command_draw_indexed(
        &self,
        _command_buffer: &mut Self::CommandBuffer,
        _draw: DrawIndexed,
    ) -> Result<()> {
        Ok(())
    }

    fn command_draw(&self, _command_buffer: &mut Self::CommandBuffer, _draw: Draw) -> Result<()> {
        Ok(())
    }

    fn create_fence(&self, _signaled: bool) -> Result<Self::Fence> {
        Ok(())
    }

    fn wait_for_fence(&self, _fence: &Self::Fence, _timeout_ns: u64) -> Result<()> {
        Ok(())
    }

    fn reset_fence(&self, _fence: &mut Self::Fence) -> Result<()> {
        Ok(())
    }

    fn create_semaphore(&self, _kind: SemaphoreKind) -> Result<Self::Semaphore> {
        Ok(())
    }

    fn wait_for_semaphore(
        &self,
        _semaphore: &Self::Semaphore,
        _value: u64,
        _timeout_ns: u64,
    ) -> Result<()> {
        Ok(())
    }

    fn semaphore_value(&self, _semaphore: &Self::Semaphore) -> Result<u64> {
        Ok(0)
    }

    fn submit(
        &self,
        _queue: QueueType,
        _info: &SubmitInfo<'_, Self>,
        _fence: &Fence<Self>,
    ) -> Result<()> {
        Ok(())
    }

    #[cfg(feature = "presentation")]
    type SurfaceTarget = ();
    #[cfg(feature = "presentation")]
    type Surface = ();
    #[cfg(feature = "presentation")]
    type Swapchain = FakeSwapchain;
    #[cfg(feature = "presentation")]
    type RenderImage = FakeRenderImage;

    #[cfg(feature = "presentation")]
    fn create_surface(&self, _target: &Self::SurfaceTarget) -> Result<Self::Surface> {
        Ok(())
    }

    #[cfg(feature = "presentation")]
    fn create_swapchain(
        &self,
        _surface: &Self::Surface,
        info: &SwapchainCreateInfo<'_>,
    ) -> Result<Self::Swapchain> {
        Ok(FakeSwapchain {
            extent: info.extent,
            format: info
                .preferred_formats
                .first()
                .copied()
                .unwrap_or(Format::Bgra8Srgb),
        })
    }

    #[cfg(feature = "presentation")]
    fn recreate_swapchain(
        &self,
        swapchain: &mut Self::Swapchain,
        _surface: &Self::Surface,
        info: &SwapchainCreateInfo<'_>,
    ) -> Result<()> {
        swapchain.extent = info.extent;
        Ok(())
    }

    #[cfg(feature = "presentation")]
    fn swapchain_extent(swapchain: &Self::Swapchain) -> Extent2D {
        swapchain.extent
    }

    #[cfg(feature = "presentation")]
    fn swapchain_format(swapchain: &Self::Swapchain) -> Format {
        swapchain.format
    }

    #[cfg(feature = "presentation")]
    fn acquire_render_image(
        &self,
        _swapchain: &mut Self::Swapchain,
        _timeout_ns: u64,
        _signal: &Self::Semaphore,
    ) -> Result<Self::RenderImage> {
        Ok(FakeRenderImage {
            image: (),
            view: (),
            index: 0,
        })
    }

    #[cfg(feature = "presentation")]
    fn render_image_parts(image: &Self::RenderImage) -> (&Self::Image, &Self::ImageView, u32) {
        (&image.image, &image.view, image.index)
    }

    #[cfg(feature = "presentation")]
    fn present(
        &self,
        _swapchain: &mut Self::Swapchain,
        _image: Self::RenderImage,
        _waits: &[&Self::Semaphore],
    ) -> Result<()> {
        self.presented_images.set(self.presented_images.get() + 1);
        Ok(())
    }

    #[cfg(feature = "presentation")]
    fn abandon_render_image(
        &self,
        _swapchain: &mut Self::Swapchain,
        _image: Self::RenderImage,
    ) -> Result<()> {
        self.abandoned_images.set(self.abandoned_images.get() + 1);
        Ok(())
    }
}

#[cfg(feature = "presentation")]
pub(crate) struct FakeSwapchain {
    extent: Extent2D,
    format: Format,
}

#[cfg(feature = "presentation")]
pub(crate) struct FakeRenderImage {
    image: (),
    view: (),
    index: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command_pool(
        device: &Device<FakeBackend>,
        flags: CommandPoolFlags,
    ) -> CommandPool<FakeBackend> {
        device
            .create_command_pool(&CommandPoolCreateInfo {
                queue: QueueType::Graphics,
                flags,
                label: None,
            })
            .expect("fake command pool creation should succeed")
    }

    fn executable(
        pool: &CommandPool<FakeBackend>,
        usage: CommandBufferUsage,
    ) -> CommandBuffer<FakeBackend> {
        let mut command_buffer = pool
            .allocate(CommandBufferLevel::Primary)
            .expect("fake command buffer allocation should succeed");
        command_buffer
            .begin(&CommandBufferBeginInfo { usage })
            .expect("begin should succeed");
        command_buffer.end().expect("end should succeed");
        command_buffer
    }

    #[test]
    fn rejects_resources_from_another_device() {
        let first = device();
        let second = device();
        let buffer = first
            .create_buffer(&BufferCreateInfo {
                size: 4,
                usage: BufferUsage::UNIFORM,
                memory: MemoryLocation::Upload,
                label: None,
            })
            .expect("fake buffer creation should succeed");

        assert!(matches!(
            second.write_buffer(&buffer, 0, &[0; 4]),
            Err(Error::DeviceMismatch { .. })
        ));
    }

    #[test]
    fn pool_reset_returns_allocated_buffers_to_initial() {
        let device = device();
        let mut pool = command_pool(&device, CommandPoolFlags::empty());
        let mut command_buffer = executable(&pool, CommandBufferUsage::empty());

        pool.reset().expect("pool reset should succeed");
        command_buffer
            .begin(&CommandBufferBeginInfo::default())
            .expect("generation change should restore initial state");
    }

    #[test]
    fn individual_reset_requires_pool_flag() {
        let device = device();
        let pool = command_pool(&device, CommandPoolFlags::empty());
        let mut command_buffer = executable(&pool, CommandBufferUsage::empty());

        assert!(matches!(
            command_buffer.reset(),
            Err(Error::InvalidDescriptor { .. })
        ));
    }

    #[test]
    fn rendering_scope_transitions_are_checked() {
        let device = device();
        let pool = command_pool(&device, CommandPoolFlags::empty());
        let mut command_buffer = pool
            .allocate(CommandBufferLevel::Primary)
            .expect("allocation should succeed");
        command_buffer
            .begin(&CommandBufferBeginInfo::default())
            .expect("begin should succeed");
        let rendering = RenderingInfo {
            render_area: Rect2D::default(),
            layer_count: 1,
            color_attachments: &[],
            depth_stencil_attachment: None,
        };

        assert!(command_buffer.end_rendering().is_err());
        assert!(
            command_buffer
                .draw(Draw {
                    first_vertex: 0,
                    vertex_count: 3,
                    first_instance: 0,
                    instance_count: 1,
                })
                .is_err()
        );
        command_buffer
            .begin_rendering(&rendering)
            .expect("first rendering scope should begin");
        command_buffer
            .draw(Draw {
                first_vertex: 0,
                vertex_count: 3,
                first_instance: 0,
                instance_count: 1,
            })
            .expect("draw should be valid inside rendering");
        assert!(command_buffer.begin_rendering(&rendering).is_err());
        assert!(command_buffer.barriers(&[], &[]).is_err());
        assert!(command_buffer.end().is_err());
        command_buffer
            .end_rendering()
            .expect("active rendering scope should end");
        command_buffer.end().expect("command buffer should end");
    }

    #[test]
    fn submission_blocks_reset_until_fence_completion() {
        let device = device();
        let mut pool = command_pool(&device, CommandPoolFlags::RESET_COMMAND_BUFFER);
        let mut command_buffer = executable(&pool, CommandBufferUsage::ONE_TIME_SUBMIT);
        let mut fence = device
            .create_fence(false)
            .expect("fence creation should succeed");

        device
            .submit(
                QueueType::Graphics,
                &SubmitInfo {
                    waits: &[],
                    command_buffers: &[&command_buffer],
                    signals: &[],
                },
                &fence,
            )
            .expect("submission should succeed");
        assert!(command_buffer.reset().is_err());
        assert!(pool.reset().is_err());
        assert!(device.reset_fence(&mut fence).is_err());

        device
            .wait_for_fence(&fence, u64::MAX)
            .expect("fence wait should complete submission");
        device
            .reset_fence(&mut fence)
            .expect("completed fence should reset");
        assert!(
            device
                .submit(
                    QueueType::Graphics,
                    &SubmitInfo {
                        waits: &[],
                        command_buffers: &[&command_buffer],
                        signals: &[],
                    },
                    &fence,
                )
                .is_err()
        );

        command_buffer
            .reset()
            .expect("completed buffer should reset");
        command_buffer
            .begin(&CommandBufferBeginInfo::default())
            .expect("reset buffer should record again");
    }

    #[test]
    fn wait_idle_completes_tracked_submissions() {
        let device = device();
        let mut pool = command_pool(&device, CommandPoolFlags::RESET_COMMAND_BUFFER);
        let mut command_buffer = executable(&pool, CommandBufferUsage::empty());
        let fence = device
            .create_fence(false)
            .expect("fence creation should succeed");

        device
            .submit(
                QueueType::Graphics,
                &SubmitInfo {
                    waits: &[],
                    command_buffers: &[&command_buffer],
                    signals: &[],
                },
                &fence,
            )
            .expect("submission should succeed");
        device.wait_idle().expect("device wait should succeed");

        command_buffer
            .reset()
            .expect("wait_idle should release command buffers");
        pool.reset()
            .expect("wait_idle should release command pools");
    }

    #[test]
    fn wait_idle_recovers_submission_after_wrappers_drop() {
        let device = device();
        let mut pool = command_pool(&device, CommandPoolFlags::RESET_COMMAND_BUFFER);
        let command_buffer = executable(&pool, CommandBufferUsage::empty());
        let fence = device
            .create_fence(false)
            .expect("fence creation should succeed");

        device
            .submit(
                QueueType::Graphics,
                &SubmitInfo {
                    waits: &[],
                    command_buffers: &[&command_buffer],
                    signals: &[],
                },
                &fence,
            )
            .expect("submission should succeed");
        drop(fence);
        drop(command_buffer);
        device.wait_idle().expect("device wait should succeed");

        pool.reset()
            .expect("registry should retain dropped submission state");
    }

    #[test]
    fn cube_view_validation_rejects_incompatible_images() {
        let device = device();
        let image = device
            .create_image(&ImageCreateInfo {
                dimension: ImageDimension::Two,
                extent: Extent3D::new(64, 32, 1),
                format: Format::Rgba8Unorm,
                usage: ImageUsage::SAMPLED,
                memory: MemoryLocation::Device,
                mip_levels: 1,
                array_layers: 6,
                samples: SampleCount::One,
                label: None,
            })
            .expect("fake image creation should succeed");

        assert!(matches!(
            device.create_image_view(
                &image,
                &ImageViewCreateInfo {
                    dimension: ImageViewDimension::Cube,
                    format: None,
                    range: ImageSubresourceRange::all(ImageAspects::COLOR, 1, 6),
                    label: None,
                },
            ),
            Err(Error::InvalidDescriptor { .. })
        ));
    }

    #[cfg(feature = "presentation")]
    #[test]
    fn render_image_present_and_abandon_consume_acquisition() {
        let device = device();
        let surface = device
            .create_surface(&())
            .expect("surface creation should succeed");
        let mut swapchain = device
            .create_swapchain(
                &surface,
                &SwapchainCreateInfo {
                    extent: Extent2D::new(640, 480),
                    image_usage: ImageUsage::COLOR_ATTACHMENT,
                    preferred_formats: &[Format::Bgra8Srgb],
                    present_mode: PresentMode::Fifo,
                    label: None,
                },
            )
            .expect("swapchain creation should succeed");
        let acquire = device
            .create_semaphore(SemaphoreKind::Binary)
            .expect("semaphore creation should succeed");

        swapchain
            .acquire_next_image(u64::MAX, &acquire)
            .expect("image acquisition should succeed")
            .abandon()
            .expect("abandon should succeed");
        swapchain
            .acquire_next_image(u64::MAX, &acquire)
            .expect("second acquisition should succeed")
            .present(&[])
            .expect("presentation should succeed");
        drop(
            swapchain
                .acquire_next_image(u64::MAX, &acquire)
                .expect("third acquisition should succeed"),
        );

        assert!(
            swapchain
                .recreate(&SwapchainCreateInfo {
                    extent: Extent2D::new(800, 600),
                    image_usage: ImageUsage::COLOR_ATTACHMENT,
                    preferred_formats: &[Format::Bgra8Srgb],
                    present_mode: PresentMode::Fifo,
                    label: None,
                })
                .is_err()
        );
        assert_eq!(device.backend.abandoned_images.get(), 1);
        assert_eq!(device.backend.presented_images.get(), 1);
    }

    #[cfg(feature = "presentation")]
    #[test]
    fn invalid_present_wait_releases_acquisition() {
        let device = device();
        let surface = device
            .create_surface(&())
            .expect("surface creation should succeed");
        let mut swapchain = device
            .create_swapchain(
                &surface,
                &SwapchainCreateInfo {
                    extent: Extent2D::new(640, 480),
                    image_usage: ImageUsage::COLOR_ATTACHMENT,
                    preferred_formats: &[Format::Bgra8Srgb],
                    present_mode: PresentMode::Fifo,
                    label: None,
                },
            )
            .expect("swapchain creation should succeed");
        let acquire = device
            .create_semaphore(SemaphoreKind::Binary)
            .expect("semaphore creation should succeed");
        let invalid_wait = device
            .create_semaphore(SemaphoreKind::Timeline { initial_value: 0 })
            .expect("timeline semaphore creation should succeed");

        let error = swapchain
            .acquire_next_image(u64::MAX, &acquire)
            .expect("image acquisition should succeed")
            .present(&[&invalid_wait])
            .expect_err("timeline presentation wait should fail");
        assert!(matches!(error, Error::InvalidDescriptor { .. }));
        assert_eq!(device.backend.abandoned_images.get(), 1);
        swapchain
            .recreate(&SwapchainCreateInfo {
                extent: Extent2D::new(800, 600),
                image_usage: ImageUsage::COLOR_ATTACHMENT,
                preferred_formats: &[Format::Bgra8Srgb],
                present_mode: PresentMode::Fifo,
                label: None,
            })
            .expect("invalid presentation should release the acquisition");
    }
}
