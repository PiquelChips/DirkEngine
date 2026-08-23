use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

use crate::{
    Backend, BindGroupDesc, BindGroupLayoutDesc, Buffer, BufferCopy, BufferDesc, BufferImageCopy,
    BufferUsages, Capabilities, Color, CommandBuffer, DependencyInfo, Error, Extent3d, Fence,
    FilterMode, GraphicsPipelineDesc, ImageCopy, ImageDesc, ImageUsages, ImageViewDesc,
    InvalidResource, PipelineLayoutDesc, PipelineStages, QueueType, Rect, RenderingInfo, Result,
    RhiCreateInfo, SampleCount, SamplerDesc, ShaderDesc, StencilOp, Submission, SurfaceCreateInfo,
    SurfaceFrame, SurfaceStatus, Swapchain, SwapchainDesc, TextureFormat, TimelinePoint,
    TimelineSemaphore, UnsupportedOperation, Viewport,
};

#[derive(Clone, Debug)]
struct TestBuffer {
    size: Arc<AtomicU64>,
}

impl TestBuffer {
    fn new(size: u64) -> Self {
        Self {
            size: Arc::new(AtomicU64::new(size)),
        }
    }
}

impl Default for TestBuffer {
    fn default() -> Self {
        Self::new(0)
    }
}

impl Buffer for TestBuffer {
    fn size(&self) -> u64 {
        self.size.load(Ordering::Acquire)
    }

    fn write(&self, offset: u64, data: &[u8]) -> Result<()> {
        let length =
            u64::try_from(data.len()).map_err(|error| Error::Backend(anyhow::Error::new(error)))?;
        if offset
            .checked_add(length)
            .is_none_or(|end| end > self.size())
        {
            return Err(InvalidResource::OutOfRange.into());
        }
        Ok(())
    }
}

#[derive(Default)]
struct TestFence(AtomicBool);

impl Fence for TestFence {
    fn wait(&self, _timeout_ns: u64) -> Result<()> {
        if self.0.load(Ordering::Acquire) {
            Ok(())
        } else {
            Err(Error::Timeout)
        }
    }

    fn reset(&self) -> Result<()> {
        self.0.store(false, Ordering::Release);
        Ok(())
    }
}

#[derive(Clone, Default)]
struct TestTimeline(Arc<AtomicU64>);

impl TimelineSemaphore for TestTimeline {
    fn wait(&self, value: u64, _timeout_ns: u64) -> Result<()> {
        if self.0.load(Ordering::Acquire) >= value {
            Ok(())
        } else {
            Err(Error::Timeout)
        }
    }

    fn value(&self) -> Result<u64> {
        Ok(self.0.load(Ordering::Acquire))
    }
}

#[derive(Clone, Debug, Default)]
struct TestResource;

#[derive(Default)]
struct TestCommandPool;

#[derive(Default)]
struct TestCommandBuffer;

#[derive(Default)]
struct TestSurfaceFrame {
    image: TestResource,
    view: TestResource,
}

#[derive(Default)]
struct TestSwapchain;

#[derive(Default)]
struct TestBackend;

impl CommandBuffer<TestBackend> for TestCommandBuffer {
    fn begin(&mut self, _label: &str, _one_time_submit: bool) -> Result<()> {
        Ok(())
    }

    fn end(&mut self) -> Result<()> {
        Ok(())
    }

    fn begin_rendering(&mut self, _info: &RenderingInfo<'_, TestBackend>) -> Result<()> {
        Ok(())
    }

    fn end_rendering(&mut self) -> Result<()> {
        Ok(())
    }

    fn set_viewport(&mut self, _viewport: Viewport) {}

    fn set_scissor(&mut self, _scissor: Rect) {}

    fn set_blend_constants(&mut self, _color: Color) {}

    fn set_stencil_reference(&mut self, _front: u32, _back: u32) {}

    fn bind_graphics_pipeline(&mut self, _pipeline: &TestResource) {}

    fn bind_groups(
        &mut self,
        _layout: &TestResource,
        _first_group: u32,
        _groups: &[&TestResource],
    ) {
    }

    fn bind_vertex_buffer(&mut self, _slot: u32, _buffer: &TestBuffer, _offset: u64) {}

    fn bind_index_buffer(
        &mut self,
        _buffer: &TestBuffer,
        _offset: u64,
        _format: crate::IndexFormat,
    ) {
    }

    fn draw(
        &mut self,
        _vertex_count: u32,
        _instance_count: u32,
        _first_vertex: u32,
        _first_instance: u32,
    ) {
    }

    fn draw_indexed(
        &mut self,
        _index_count: u32,
        _instance_count: u32,
        _first_index: u32,
        _vertex_offset: i32,
        _first_instance: u32,
    ) {
    }

    fn copy_buffer(&mut self, _src: &TestBuffer, _dst: &TestBuffer, _regions: &[BufferCopy]) {}

    fn copy_buffer_to_image(
        &mut self,
        _src: &TestBuffer,
        _dst: &TestResource,
        _regions: &[BufferImageCopy],
    ) {
    }

    fn copy_image(&mut self, _src: &TestResource, _dst: &TestResource, _regions: &[ImageCopy]) {}

    fn blit_image(
        &mut self,
        _src: &TestResource,
        _dst: &TestResource,
        _regions: &[crate::ImageBlit],
        _filter: FilterMode,
    ) -> Result<()> {
        Ok(())
    }

    fn barrier(&mut self, _dependency: &DependencyInfo<'_, TestBackend>) {}
}

impl SurfaceFrame<TestBackend> for TestSurfaceFrame {
    fn image(&self) -> &TestResource {
        &self.image
    }

    fn view(&self) -> &TestResource {
        &self.view
    }

    fn format(&self) -> TextureFormat {
        TextureFormat::Rgba8Unorm
    }

    fn extent(&self) -> Extent3d {
        Extent3d::new_2d(1, 1)
    }

    fn status(&self) -> SurfaceStatus {
        SurfaceStatus::Optimal
    }
}

impl Swapchain<TestBackend> for TestSwapchain {
    fn format(&self) -> TextureFormat {
        TextureFormat::Rgba8Unorm
    }

    fn extent(&self) -> Extent3d {
        Extent3d::new_2d(1, 1)
    }

    fn acquire(&mut self) -> Result<TestSurfaceFrame> {
        Ok(TestSurfaceFrame::default())
    }

    fn resize(&mut self, _width: u32, _height: u32) -> Result<()> {
        Ok(())
    }

    fn present(&mut self, frame: TestSurfaceFrame) -> Result<SurfaceStatus> {
        Ok(frame.status())
    }
}

impl Backend for TestBackend {
    type Buffer = TestBuffer;
    type Image = TestResource;
    type ImageView = TestResource;
    type Sampler = TestResource;
    type Shader = TestResource;
    type BindGroupLayout = TestResource;
    type BindGroup = TestResource;
    type PipelineLayout = TestResource;
    type GraphicsPipeline = TestResource;
    type CommandPool = TestCommandPool;
    type CommandBuffer = TestCommandBuffer;
    type Fence = TestFence;
    type TimelineSemaphore = TestTimeline;
    type Surface = TestResource;
    type Swapchain = TestSwapchain;
    type SurfaceFrame = TestSurfaceFrame;

    fn new(_info: &RhiCreateInfo<'_>) -> Result<Self> {
        Ok(Self)
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            max_samples: SampleCount::Eight,
            max_sampler_anisotropy: 1,
            min_uniform_buffer_offset_alignment: 256,
            min_storage_buffer_offset_alignment: 16,
            dedicated_compute_queue: false,
            dedicated_copy_queue: false,
        }
    }

    fn supported_depth_formats(&self) -> &'static [TextureFormat] {
        &[TextureFormat::Depth32Float, TextureFormat::Depth16Unorm]
    }

    fn wait_idle(&self) -> Result<()> {
        Ok(())
    }

    fn collect_garbage(&self) -> Result<()> {
        Ok(())
    }

    fn create_buffer(&self, desc: &BufferDesc<'_>) -> Result<TestBuffer> {
        if desc.size == 0 {
            return Err(InvalidResource::Empty.into());
        }
        Ok(TestBuffer::new(desc.size))
    }

    fn create_image(&self, _desc: &ImageDesc<'_>) -> Result<TestResource> {
        Ok(TestResource)
    }

    fn create_image_view(&self, _desc: &ImageViewDesc<'_, Self>) -> Result<TestResource> {
        Ok(TestResource)
    }

    fn create_sampler(&self, _desc: &SamplerDesc<'_>) -> Result<TestResource> {
        Ok(TestResource)
    }

    fn create_shader(&self, _desc: &ShaderDesc<'_>) -> Result<TestResource> {
        Ok(TestResource)
    }

    fn create_bind_group_layout(&self, _desc: &BindGroupLayoutDesc<'_>) -> Result<TestResource> {
        Ok(TestResource)
    }

    fn create_bind_group(&self, _desc: &BindGroupDesc<'_, Self>) -> Result<TestResource> {
        Ok(TestResource)
    }

    fn create_pipeline_layout(&self, _desc: &PipelineLayoutDesc<'_, Self>) -> Result<TestResource> {
        Ok(TestResource)
    }

    fn create_graphics_pipeline(
        &self,
        _desc: &GraphicsPipelineDesc<'_, Self>,
    ) -> Result<TestResource> {
        Ok(TestResource)
    }

    fn create_command_pool(&self, _queue: QueueType) -> Result<TestCommandPool> {
        Ok(TestCommandPool)
    }

    fn create_command_buffer(&self, _pool: &mut TestCommandPool) -> Result<TestCommandBuffer> {
        Ok(TestCommandBuffer)
    }

    fn create_fence(&self, signaled: bool) -> Result<TestFence> {
        Ok(TestFence(AtomicBool::new(signaled)))
    }

    fn create_timeline_semaphore(&self, initial_value: u64) -> Result<TestTimeline> {
        Ok(TestTimeline(Arc::new(AtomicU64::new(initial_value))))
    }

    fn submit(&self, _queue: QueueType, _submission: &Submission<'_, Self>) -> Result<()> {
        Ok(())
    }

    fn create_surface(&self, _info: SurfaceCreateInfo<'_>) -> Result<TestResource> {
        Ok(TestResource)
    }

    fn create_swapchain(&self, _desc: &SwapchainDesc<'_, Self>) -> Result<TestSwapchain> {
        Ok(TestSwapchain)
    }
}

#[test]
fn usage_flags_compose_without_backend_values() {
    const DEFAULT: BufferUsages = BufferUsages::COPY_DST.union(BufferUsages::VERTEX);
    let usage = BufferUsages::COPY_DST | BufferUsages::VERTEX;

    assert_eq!(usage, DEFAULT);
    assert!(usage.contains(BufferUsages::COPY_DST));
    assert!(usage.contains(BufferUsages::VERTEX));
    assert!(!usage.contains(BufferUsages::UNIFORM));
    assert!(BufferUsages::NONE.is_empty());
    assert!(BufferUsages::from_bits(usage.bits()).is_some());
    assert!(BufferUsages::from_bits(u32::MAX).is_none());
}

#[test]
fn image_usage_flags_preserve_all_requested_roles() {
    let usage = ImageUsages::SAMPLED | ImageUsages::COLOR_ATTACHMENT | ImageUsages::COPY_SRC;

    assert!(usage.contains(ImageUsages::SAMPLED | ImageUsages::COPY_SRC));
    assert!(usage.contains(ImageUsages::COLOR_ATTACHMENT));
}

#[test]
fn semantic_types_do_not_encode_backend_constants() {
    assert_eq!(Extent3d::new_2d(1920, 1080).depth, 1);
    assert_eq!(SampleCount::Four as u8, 4);
    assert_eq!(
        PipelineStages::ALL,
        PipelineStages::COPY
            | PipelineStages::VERTEX
            | PipelineStages::FRAGMENT
            | PipelineStages::COLOR_OUTPUT
            | PipelineStages::COMPUTE
    );
    assert!(SampleCount::Four < SampleCount::Eight);
}

#[test]
#[allow(
    clippy::float_cmp,
    reason = "constructors must reproduce these constants exactly"
)]
fn semantic_helpers_build_expected_values() {
    let viewport = Viewport::dimensions(0.0, 0.0, 1280.0, 720.0);
    assert_eq!(viewport.min_depth, 0.0);
    assert_eq!(viewport.max_depth, 1.0);

    let scissor = Rect::new(4, 8, 640, 360);
    assert_eq!((scissor.x, scissor.y), (4, 8));

    assert_eq!(Color::TRANSPARENT.a, 0.0);
    assert_eq!(Color::BLACK, Color::new(0.0, 0.0, 0.0, 1.0));
    assert_eq!(
        Color::WHITE,
        Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        }
    );
}

#[test]
fn resources_own_their_stateful_operations() -> Result<()> {
    let buffer = TestBuffer::new(12);
    assert_eq!(buffer.size(), 12);
    buffer.write(8, &[1, 2, 3, 4])?;
    assert!(matches!(
        buffer.write(9, &[1, 2, 3, 4]),
        Err(Error::InvalidResource(InvalidResource::OutOfRange))
    ));

    let fence = TestFence::default();
    assert!(matches!(fence.wait(0), Err(Error::Timeout)));
    fence.0.store(true, Ordering::Release);
    fence.wait(u64::MAX)?;
    assert!(fence.0.load(Ordering::Acquire));
    fence.reset()?;
    assert!(!fence.0.load(Ordering::Acquire));
    assert!(matches!(fence.wait(u64::MAX), Err(Error::Timeout)));

    let timeline = TestTimeline::default();
    assert!(matches!(timeline.wait(42, 0), Err(Error::Timeout)));
    timeline.0.store(42, Ordering::Release);
    timeline.wait(42, u64::MAX)?;
    assert_eq!(timeline.value()?, 42);
    assert!(matches!(timeline.wait(43, u64::MAX), Err(Error::Timeout)));
    Ok(())
}

#[test]
fn backend_errors_keep_anyhow_context() {
    let source = anyhow::anyhow!("native allocation failed").context("creating buffer");
    let error = Error::from(source);

    assert!(matches!(error, Error::Backend(_)));
    assert_eq!(error.to_string(), "graphics backend error: creating buffer");

    let Error::Backend(inner) = &error else {
        unreachable!("classified as a backend error above");
    };
    let causes: Vec<_> = inner.chain().map(ToString::to_string).collect();
    assert_eq!(causes, ["creating buffer", "native allocation failed"]);
}

#[test]
fn typed_errors_describe_recoverable_conditions() {
    assert_eq!(
        Error::from(UnsupportedOperation::ImageBlit).to_string(),
        "unsupported RHI operation: image blits are not supported by this backend"
    );
    let language_error = Error::from(UnsupportedOperation::ShaderSource(
        crate::ShaderLanguage::Msl,
    ));
    assert_eq!(
        language_error.to_string(),
        "unsupported RHI operation: Metal Shading Language shader source is not supported by this backend"
    );
    assert_eq!(
        Error::from(InvalidResource::ForeignInstance).to_string(),
        "invalid resource description: resource belongs to a different RHI instance"
    );
}

#[test]
fn shader_sources_report_their_language() {
    assert_eq!(
        crate::ShaderSource::SpirV(&[0; 4]).language(),
        crate::ShaderLanguage::SpirV
    );
    assert_eq!(
        crate::ShaderSource::Msl("void main() {}").language(),
        crate::ShaderLanguage::Msl
    );
}

#[test]
#[allow(clippy::float_cmp, reason = "default bias must be exactly zero")]
fn stencil_state_defaults_to_keep_operations() {
    let face = crate::StencilFaceState {
        compare: crate::CompareOp::Always,
        fail_op: StencilOp::Keep,
        depth_fail_op: StencilOp::default(),
        pass_op: StencilOp::Replace,
    };
    assert_eq!(face.depth_fail_op, StencilOp::Keep);
    assert_eq!(crate::DepthBiasState::default().constant_factor, 0.0);
}

#[test]
fn backend_contract_accepts_borrowed_descriptors_and_submission() -> Result<()> {
    let backend = TestBackend::new(&RhiCreateInfo {
        engine_name: "test",
        engine_version: (0, 1, 0),
        application_name: "test",
        application_version: (0, 1, 0),
        validation: false,
        compatible_surface: None,
    })?;
    let mut pool = backend.create_command_pool(QueueType::Graphics)?;
    let command_buffer = backend.create_command_buffer(&mut pool)?;
    let command_buffers = [&command_buffer];
    let surface_frames: &[&TestSurfaceFrame] = &[];
    let wait_timelines: &[TimelinePoint<'_, TestBackend>] = &[];
    let signal_timelines: &[TimelinePoint<'_, TestBackend>] = &[];
    let submission = Submission {
        command_buffers: &command_buffers,
        surface_frames,
        wait_timelines,
        signal_timelines,
        fence: None,
    };

    backend.submit(QueueType::Graphics, &submission)
}
