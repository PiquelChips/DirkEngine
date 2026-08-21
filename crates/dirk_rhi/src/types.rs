use crate::flags::define_flags;

/// Semantic queue used for command execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum QueueType {
    /// Rendering and general-purpose graphics commands.
    Graphics,
    /// Compute-only work.
    Compute,
    /// Resource copies and uploads.
    Copy,
}

/// Three-dimensional unsigned extent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Extent3d {
    /// Width in pixels or buffer elements.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Number of depth slices.
    ///
    /// Array layers are specified by the descriptor that owns this extent.
    pub depth: u32,
}

/// Three-dimensional unsigned texel origin.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Origin3d {
    /// Horizontal texel offset.
    pub x: u32,
    /// Vertical texel offset.
    pub y: u32,
    /// Depth-slice offset.
    pub z: u32,
}

impl Extent3d {
    /// Creates a two-dimensional extent with a depth of one.
    #[must_use]
    pub const fn new_2d(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            depth: 1,
        }
    }
}

/// Supported texel and vertex formats.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Format {
    /// Four-channel normalized RGBA.
    Rgba8Unorm,
    /// Four-channel sRGB RGBA.
    Rgba8Srgb,
    /// Four-channel normalized BGRA.
    Bgra8Unorm,
    /// Four-channel sRGB BGRA.
    Bgra8Srgb,
    /// Two 32-bit floating-point channels.
    Rg32Float,
    /// Three 32-bit floating-point channels.
    Rgb32Float,
    /// 16-bit normalized depth.
    Depth16Unorm,
    /// 24-bit normalized depth with 8-bit stencil.
    Depth24UnormStencil8,
    /// 32-bit floating-point depth.
    Depth32Float,
    /// 32-bit floating-point depth with 8-bit stencil.
    Depth32FloatStencil8,
}

/// Texture sample count.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum SampleCount {
    /// One sample per pixel.
    #[default]
    One = 1,
    /// Two samples per pixel.
    Two = 2,
    /// Four samples per pixel.
    Four = 4,
    /// Eight samples per pixel.
    Eight = 8,
}

/// Preferred memory access pattern for a resource.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MemoryDomain {
    /// GPU-local memory.
    Device,
    /// CPU-written memory used to upload data.
    Upload,
    /// CPU-readable memory used to retrieve GPU results.
    Readback,
}

define_flags! {
    /// Permitted uses of a buffer.
    pub struct BufferUsages(u32) {
        /// Source of a copy operation.
        const COPY_SRC = 1 << 0;
        /// Destination of a copy operation.
        const COPY_DST = 1 << 1;
        /// Vertex input.
        const VERTEX = 1 << 2;
        /// Index input.
        const INDEX = 1 << 3;
        /// Uniform data.
        const UNIFORM = 1 << 4;
        /// Shader storage data.
        const STORAGE = 1 << 5;
    }
}

define_flags! {
    /// Permitted uses of an image.
    pub struct ImageUsages(u32) {
        /// Source of a copy operation.
        const COPY_SRC = 1 << 0;
        /// Destination of a copy operation.
        const COPY_DST = 1 << 1;
        /// Sampled by a shader.
        const SAMPLED = 1 << 2;
        /// Read or written as shader storage.
        const STORAGE = 1 << 3;
        /// Color render target.
        const COLOR_ATTACHMENT = 1 << 4;
        /// Depth or stencil render target.
        const DEPTH_STENCIL_ATTACHMENT = 1 << 5;
        /// Memory may be transient outside a render pass.
        const TRANSIENT_ATTACHMENT = 1 << 6;
        /// Presented to a display surface.
        const PRESENT = 1 << 7;
    }
}

define_flags! {
    /// Shader stages visible to a binding.
    pub struct ShaderStages(u32) {
        /// Vertex stage.
        const VERTEX = 1 << 0;
        /// Fragment stage.
        const FRAGMENT = 1 << 1;
        /// Compute stage.
        const COMPUTE = 1 << 2;
    }
}

define_flags! {
    /// Pipeline stages used to scope a submission wait.
    pub struct PipelineStages(u32) {
        /// Transfer commands.
        const COPY = 1 << 0;
        /// Vertex processing.
        const VERTEX = 1 << 1;
        /// Fragment processing.
        const FRAGMENT = 1 << 2;
        /// Color output.
        const COLOR_OUTPUT = 1 << 3;
        /// Compute processing.
        const COMPUTE = 1 << 4;
    }
}

define_flags! {
    /// Image aspects selected by a view or barrier.
    pub struct ImageAspects(u8) {
        /// Color data.
        const COLOR = 1 << 0;
        /// Depth data.
        const DEPTH = 1 << 1;
        /// Stencil data.
        const STENCIL = 1 << 2;
    }
}

/// Image dimensionality exposed through a view.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ImageViewType {
    /// A two-dimensional image.
    #[default]
    TwoD,
    /// An array of two-dimensional images.
    TwoDArray,
    /// A cube image.
    Cube,
}

/// Shader stage implemented by a module.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShaderStage {
    /// Vertex shader.
    Vertex,
    /// Fragment shader.
    Fragment,
    /// Compute shader.
    Compute,
}

/// Shader-visible resource type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BindingType {
    /// Read-only uniform buffer.
    UniformBuffer,
    /// Read-write storage buffer.
    StorageBuffer,
    /// Sampled image and sampler pair.
    SampledImage,
    /// Read-write storage image.
    StorageImage,
}

/// Primitive topology consumed by a graphics pipeline.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PrimitiveTopology {
    /// Independent triangles.
    #[default]
    TriangleList,
    /// Connected triangle strip.
    TriangleStrip,
    /// Independent lines.
    LineList,
}

/// Face winding considered front-facing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FrontFace {
    /// Counter-clockwise winding.
    #[default]
    CounterClockwise,
    /// Clockwise winding.
    Clockwise,
}

/// Triangle face culling mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CullMode {
    /// Do not cull triangles.
    None,
    /// Cull front-facing triangles.
    Front,
    /// Cull back-facing triangles.
    #[default]
    Back,
}

/// Depth or stencil comparison operation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CompareOp {
    /// Comparison always fails.
    Never,
    /// Incoming value is less than stored value.
    #[default]
    Less,
    /// Incoming value is less than or equal to stored value.
    LessEqual,
    /// Incoming value is equal to stored value.
    Equal,
    /// Incoming value is greater than stored value.
    Greater,
    /// Comparison always succeeds.
    Always,
}

/// Index element width.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IndexFormat {
    /// 16-bit unsigned indices.
    Uint16,
    /// 32-bit unsigned indices.
    Uint32,
}

/// Texture filtering mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FilterMode {
    /// Nearest texel.
    Nearest,
    /// Linear interpolation.
    #[default]
    Linear,
}

/// Texture address mode outside normalized coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AddressMode {
    /// Clamp coordinates to the edge.
    #[default]
    ClampToEdge,
    /// Repeat the image.
    Repeat,
    /// Repeat and mirror alternate copies.
    MirrorRepeat,
}

/// Current semantic use of an image.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ImageState {
    /// Contents are unavailable.
    #[default]
    Undefined,
    /// Copy source.
    CopySource,
    /// Copy destination.
    CopyDestination,
    /// Shader-readable image.
    ShaderRead,
    /// Shader-readable and writable image.
    ShaderWrite,
    /// Color attachment.
    ColorAttachment,
    /// Depth/stencil attachment.
    DepthStencilAttachment,
    /// Ready for presentation.
    Present,
}

/// Rectangle in framebuffer coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rect {
    /// Horizontal origin.
    pub x: i32,
    /// Vertical origin.
    pub y: i32,
    /// Rectangle width.
    pub width: u32,
    /// Rectangle height.
    pub height: u32,
}

/// Floating-point viewport and depth range.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    /// Horizontal origin.
    pub x: f32,
    /// Vertical origin.
    pub y: f32,
    /// Viewport width.
    pub width: f32,
    /// Viewport height.
    pub height: f32,
    /// Minimum depth.
    pub min_depth: f32,
    /// Maximum depth.
    pub max_depth: f32,
}

/// Per-vertex input rate.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum VertexStepMode {
    /// Advance once per vertex.
    #[default]
    Vertex,
    /// Advance once per instance.
    Instance,
}

/// Vertex attribute description.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VertexAttribute {
    /// Shader location.
    pub location: u32,
    /// Attribute format.
    pub format: Format,
    /// Byte offset within one vertex.
    pub offset: u32,
}

/// Vertex-buffer input layout.
#[derive(Clone, Copy, Debug)]
pub struct VertexBufferLayout<'a> {
    /// Byte stride between elements.
    pub stride: u32,
    /// Element advancement mode.
    pub step_mode: VertexStepMode,
    /// Attributes sourced from this buffer.
    pub attributes: &'a [VertexAttribute],
}

/// Color clear value.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Color {
    /// Red component.
    pub r: f32,
    /// Green component.
    pub g: f32,
    /// Blue component.
    pub b: f32,
    /// Alpha component.
    pub a: f32,
}

/// Attachment load operation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LoadOp<T> {
    /// Preserve existing contents.
    Load,
    /// Clear to the supplied value.
    Clear(T),
    /// Existing contents are irrelevant.
    DontCare,
}

/// Attachment store operation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum StoreOp {
    /// Preserve rendered contents.
    #[default]
    Store,
    /// Rendered contents may be discarded.
    DontCare,
}

/// Surface operation result that still produced a usable frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SurfaceStatus {
    /// Surface remains optimal.
    #[default]
    Optimal,
    /// Frame is usable, but the surface should soon be recreated.
    Suboptimal,
}

/// Presentation mode requested when creating a swapchain.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PresentMode {
    /// Wait for vertical blanking and queue frames in order.
    #[default]
    Fifo,
    /// Prefer low-latency queued presentation when supported.
    Mailbox,
    /// Present immediately without synchronizing to vertical blanking.
    Immediate,
}
