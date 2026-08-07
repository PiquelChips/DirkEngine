//! Backend-neutral descriptors and value types.

use crate::{Error, Result, flags::define_flags};

define_flags! {
    /// Ways a buffer may be used.
    pub struct BufferUsage(u32) {
        /// Source of a transfer operation.
        const TRANSFER_SRC = 1 << 0;
        /// Destination of a transfer operation.
        const TRANSFER_DST = 1 << 1;
        /// Uniform buffer binding.
        const UNIFORM = 1 << 2;
        /// Storage buffer binding.
        const STORAGE = 1 << 3;
        /// Vertex buffer binding.
        const VERTEX = 1 << 4;
        /// Index buffer binding.
        const INDEX = 1 << 5;
    }
}

define_flags! {
    /// Ways an image may be used.
    pub struct ImageUsage(u32) {
        /// Source of a transfer operation.
        const TRANSFER_SRC = 1 << 0;
        /// Destination of a transfer operation.
        const TRANSFER_DST = 1 << 1;
        /// Sampled by a shader.
        const SAMPLED = 1 << 2;
        /// Read or written as a storage image.
        const STORAGE = 1 << 3;
        /// Color rendering attachment.
        const COLOR_ATTACHMENT = 1 << 4;
        /// Depth or stencil rendering attachment.
        const DEPTH_STENCIL_ATTACHMENT = 1 << 5;
        /// Short-lived rendering attachment.
        const TRANSIENT_ATTACHMENT = 1 << 6;
    }
}

define_flags! {
    /// Image aspects selected by a view or operation.
    pub struct ImageAspects(u8) {
        /// Color aspect.
        const COLOR = 1 << 0;
        /// Depth aspect.
        const DEPTH = 1 << 1;
        /// Stencil aspect.
        const STENCIL = 1 << 2;
    }
}

define_flags! {
    /// Shader stages that can access a resource.
    pub struct ShaderStages(u8) {
        /// Vertex stage.
        const VERTEX = 1 << 0;
        /// Fragment stage.
        const FRAGMENT = 1 << 1;
        /// Compute stage.
        const COMPUTE = 1 << 2;
    }
}

define_flags! {
    /// Pipeline stages used by synchronization barriers.
    pub struct PipelineStages(u32) {
        /// The start of submitted work.
        const TOP = 1 << 0;
        /// Vertex shader work.
        const VERTEX_SHADER = 1 << 1;
        /// Fragment shader work.
        const FRAGMENT_SHADER = 1 << 2;
        /// Compute shader work.
        const COMPUTE_SHADER = 1 << 3;
        /// Early depth and stencil tests.
        const EARLY_DEPTH_STENCIL = 1 << 4;
        /// Late depth and stencil tests.
        const LATE_DEPTH_STENCIL = 1 << 5;
        /// Color attachment output.
        const COLOR_OUTPUT = 1 << 6;
        /// Transfer operations.
        const TRANSFER = 1 << 7;
        /// The end of submitted work.
        const BOTTOM = 1 << 8;
        /// Every command stage.
        const ALL_COMMANDS = 1 << 9;
    }
}

define_flags! {
    /// Resource accesses used by synchronization barriers.
    pub struct AccessTypes(u32) {
        /// Uniform buffer reads.
        const UNIFORM_READ = 1 << 0;
        /// Shader reads.
        const SHADER_READ = 1 << 1;
        /// Shader writes.
        const SHADER_WRITE = 1 << 2;
        /// Color attachment reads.
        const COLOR_ATTACHMENT_READ = 1 << 3;
        /// Color attachment writes.
        const COLOR_ATTACHMENT_WRITE = 1 << 4;
        /// Depth or stencil attachment reads.
        const DEPTH_STENCIL_READ = 1 << 5;
        /// Depth or stencil attachment writes.
        const DEPTH_STENCIL_WRITE = 1 << 6;
        /// Transfer reads.
        const TRANSFER_READ = 1 << 7;
        /// Transfer writes.
        const TRANSFER_WRITE = 1 << 8;
        /// Host reads.
        const HOST_READ = 1 << 9;
        /// Host writes.
        const HOST_WRITE = 1 << 10;
        /// General memory reads.
        const MEMORY_READ = 1 << 11;
        /// General memory writes.
        const MEMORY_WRITE = 1 << 12;
    }
}

define_flags! {
    /// Color channels written by a graphics pipeline.
    pub struct ColorWrites(u8) {
        /// Red channel.
        const RED = 1 << 0;
        /// Green channel.
        const GREEN = 1 << 1;
        /// Blue channel.
        const BLUE = 1 << 2;
        /// Alpha channel.
        const ALPHA = 1 << 3;
    }
}

impl ColorWrites {
    /// All color channels.
    pub const ALL: Self =
        Self(Self::RED.bits() | Self::GREEN.bits() | Self::BLUE.bits() | Self::ALPHA.bits());
}

/// Queue capability used for command allocation and submission.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum QueueType {
    /// Graphics and transfer commands.
    Graphics,
    /// Compute commands.
    Compute,
    /// Transfer commands.
    Transfer,
}

/// Preferred placement and host visibility for an allocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MemoryLocation {
    /// Device-local memory.
    Device,
    /// Host-writable memory intended for uploads.
    Upload,
    /// Host-readable memory intended for readback.
    Readback,
}

/// Two-dimensional unsigned extent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Extent2D {
    /// Width in texels or pixels.
    pub width: u32,
    /// Height in texels or pixels.
    pub height: u32,
}

impl Extent2D {
    /// Creates a two-dimensional extent.
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Returns the complete mip chain length for this extent.
    #[must_use]
    pub const fn max_mip_levels(self) -> u32 {
        let longest = if self.width > self.height {
            self.width
        } else {
            self.height
        };
        u32::BITS - longest.leading_zeros()
    }

    #[cfg(feature = "presentation")]
    pub(crate) const fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }
}

/// Three-dimensional unsigned extent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Extent3D {
    /// Width in texels.
    pub width: u32,
    /// Height in texels.
    pub height: u32,
    /// Depth in texels.
    pub depth: u32,
}

impl Extent3D {
    /// Creates a three-dimensional extent.
    #[must_use]
    pub const fn new(width: u32, height: u32, depth: u32) -> Self {
        Self {
            width,
            height,
            depth,
        }
    }

    pub(crate) const fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0 || self.depth == 0
    }
}

/// Pixel and texel formats understood by every backend.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Format {
    /// Four 8-bit normalized color channels.
    Rgba8Unorm,
    /// Four 8-bit normalized sRGB color channels.
    Rgba8Srgb,
    /// BGRA ordered 8-bit normalized color channels.
    Bgra8Unorm,
    /// BGRA ordered 8-bit normalized sRGB color channels.
    Bgra8Srgb,
    /// Two 32-bit floating-point channels.
    Rg32Float,
    /// Three 32-bit floating-point channels.
    Rgb32Float,
    /// Four 32-bit floating-point channels.
    Rgba32Float,
    /// A 16-bit normalized depth channel.
    Depth16Unorm,
    /// A 32-bit floating-point depth channel.
    Depth32Float,
    /// A 24-bit normalized depth channel and 8-bit stencil channel.
    Depth24UnormStencil8,
    /// A 32-bit floating-point depth channel and 8-bit stencil channel.
    Depth32FloatStencil8,
}

impl Format {
    /// Returns the aspects stored by this format.
    #[must_use]
    pub const fn aspects(self) -> ImageAspects {
        match self {
            Self::Depth16Unorm | Self::Depth32Float => ImageAspects::DEPTH,
            Self::Depth24UnormStencil8 | Self::Depth32FloatStencil8 => {
                ImageAspects(ImageAspects::DEPTH.bits() | ImageAspects::STENCIL.bits())
            }
            Self::Rgba8Unorm
            | Self::Rgba8Srgb
            | Self::Bgra8Unorm
            | Self::Bgra8Srgb
            | Self::Rg32Float
            | Self::Rgb32Float
            | Self::Rgba32Float => ImageAspects::COLOR,
        }
    }
}

/// Number of samples stored per pixel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SampleCount {
    /// One sample per pixel.
    #[default]
    One,
    /// Two samples per pixel.
    Two,
    /// Four samples per pixel.
    Four,
    /// Eight samples per pixel.
    Eight,
    /// Sixteen samples per pixel.
    Sixteen,
    /// Thirty-two samples per pixel.
    ThirtyTwo,
    /// Sixty-four samples per pixel.
    SixtyFour,
}

/// Image dimensionality.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ImageDimension {
    /// One-dimensional image.
    One,
    /// Two-dimensional image.
    #[default]
    Two,
    /// Three-dimensional image.
    Three,
}

/// Image view dimensionality.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ImageViewDimension {
    /// One-dimensional view.
    One,
    /// Two-dimensional view.
    #[default]
    Two,
    /// Three-dimensional view.
    Three,
    /// Cube view.
    Cube,
    /// One-dimensional array view.
    OneArray,
    /// Two-dimensional array view.
    TwoArray,
    /// Cube array view.
    CubeArray,
}

/// Description of an owned buffer.
#[derive(Clone, Copy, Debug)]
pub struct BufferCreateInfo<'a> {
    /// Buffer size in bytes.
    pub size: u64,
    /// All ways the buffer will be used.
    pub usage: BufferUsage,
    /// Preferred memory placement.
    pub memory: MemoryLocation,
    /// Optional diagnostic label.
    pub label: Option<&'a str>,
}

impl BufferCreateInfo<'_> {
    /// Validates values that are independent of a concrete backend.
    pub fn validate(&self) -> Result<()> {
        if self.size == 0 {
            return Err(Error::invalid_descriptor("buffer", "size must be non-zero"));
        }
        if self.usage.is_empty() {
            return Err(Error::invalid_descriptor(
                "buffer",
                "at least one usage must be selected",
            ));
        }
        Ok(())
    }
}

/// Description of an owned image.
#[derive(Clone, Copy, Debug)]
pub struct ImageCreateInfo<'a> {
    /// Image dimensionality.
    pub dimension: ImageDimension,
    /// Image extent.
    pub extent: Extent3D,
    /// Texel format.
    pub format: Format,
    /// All ways the image will be used.
    pub usage: ImageUsage,
    /// Preferred memory placement.
    pub memory: MemoryLocation,
    /// Number of mip levels.
    pub mip_levels: u32,
    /// Number of array layers.
    pub array_layers: u32,
    /// Number of samples per pixel.
    pub samples: SampleCount,
    /// Optional diagnostic label.
    pub label: Option<&'a str>,
}

impl ImageCreateInfo<'_> {
    /// Validates values that are independent of a concrete backend.
    pub fn validate(&self) -> Result<()> {
        if self.extent.is_empty() {
            return Err(Error::invalid_descriptor(
                "image",
                "every extent dimension must be non-zero",
            ));
        }
        if self.usage.is_empty() {
            return Err(Error::invalid_descriptor(
                "image",
                "at least one usage must be selected",
            ));
        }
        if self.mip_levels == 0 {
            return Err(Error::invalid_descriptor(
                "image",
                "mip level count must be non-zero",
            ));
        }
        if self.array_layers == 0 {
            return Err(Error::invalid_descriptor(
                "image",
                "array layer count must be non-zero",
            ));
        }
        if self.samples != SampleCount::One && self.mip_levels != 1 {
            return Err(Error::invalid_descriptor(
                "image",
                "multisampled images must have exactly one mip level",
            ));
        }
        Ok(())
    }
}

/// A contiguous image subresource range.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ImageSubresourceRange {
    /// Selected image aspects.
    pub aspects: ImageAspects,
    /// First selected mip level.
    pub base_mip_level: u32,
    /// Number of selected mip levels.
    pub mip_level_count: u32,
    /// First selected array layer.
    pub base_array_layer: u32,
    /// Number of selected array layers.
    pub array_layer_count: u32,
}

impl ImageSubresourceRange {
    /// Selects all subresources represented by the supplied counts.
    #[must_use]
    pub const fn all(aspects: ImageAspects, mip_levels: u32, array_layers: u32) -> Self {
        Self {
            aspects,
            base_mip_level: 0,
            mip_level_count: mip_levels,
            base_array_layer: 0,
            array_layer_count: array_layers,
        }
    }
}

/// Description of a view into an image.
#[derive(Clone, Copy, Debug)]
pub struct ImageViewCreateInfo<'a> {
    /// View dimensionality.
    pub dimension: ImageViewDimension,
    /// Optional reinterpretation format.
    pub format: Option<Format>,
    /// Selected image subresources.
    pub range: ImageSubresourceRange,
    /// Optional diagnostic label.
    pub label: Option<&'a str>,
}

impl ImageViewCreateInfo<'_> {
    /// Validates values that are independent of a concrete backend.
    pub fn validate(&self) -> Result<()> {
        if self.range.aspects.is_empty() {
            return Err(Error::invalid_descriptor(
                "image view",
                "at least one aspect must be selected",
            ));
        }
        if self.range.mip_level_count == 0 || self.range.array_layer_count == 0 {
            return Err(Error::invalid_descriptor(
                "image view",
                "subresource counts must be non-zero",
            ));
        }
        Ok(())
    }
}

/// Texture filtering mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Filter {
    /// Select the nearest texel.
    Nearest,
    /// Linearly interpolate neighboring texels.
    #[default]
    Linear,
}

/// Texture coordinate addressing mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AddressMode {
    /// Repeat at integer boundaries.
    #[default]
    Repeat,
    /// Mirror at integer boundaries.
    MirrorRepeat,
    /// Clamp to the edge texel.
    ClampToEdge,
}

/// Description of a texture sampler.
#[derive(Clone, Copy, Debug)]
pub struct SamplerCreateInfo<'a> {
    /// Magnification filter.
    pub mag_filter: Filter,
    /// Minification filter.
    pub min_filter: Filter,
    /// Mipmap filter.
    pub mipmap_filter: Filter,
    /// U coordinate addressing.
    pub address_u: AddressMode,
    /// V coordinate addressing.
    pub address_v: AddressMode,
    /// W coordinate addressing.
    pub address_w: AddressMode,
    /// Smallest accessible mip level.
    pub lod_min: f32,
    /// Largest accessible mip level.
    pub lod_max: f32,
    /// Enables the backend's maximum supported anisotropy.
    pub anisotropy: bool,
    /// Optional diagnostic label.
    pub label: Option<&'a str>,
}

impl Default for SamplerCreateInfo<'_> {
    fn default() -> Self {
        Self {
            mag_filter: Filter::Linear,
            min_filter: Filter::Linear,
            mipmap_filter: Filter::Linear,
            address_u: AddressMode::Repeat,
            address_v: AddressMode::Repeat,
            address_w: AddressMode::Repeat,
            lod_min: 0.0,
            lod_max: 32.0,
            anisotropy: true,
            label: None,
        }
    }
}

/// Portable shader input accepted by current `DirkEngine` backends.
///
/// A backend may need to validate or translate this source. Creation can fail
/// when the source cannot be represented by the target shading language.
#[derive(Clone, Copy, Debug)]
pub enum ShaderSource<'a> {
    /// SPIR-V words. Backends that do not consume SPIR-V directly may translate it.
    SpirV(&'a [u32]),
}

/// Description of a shader module.
#[derive(Clone, Copy, Debug)]
pub struct ShaderModuleCreateInfo<'a> {
    /// Shader source code.
    pub source: ShaderSource<'a>,
    /// Optional diagnostic label.
    pub label: Option<&'a str>,
}

impl ShaderModuleCreateInfo<'_> {
    /// Validates values that are independent of a concrete backend.
    pub fn validate(&self) -> Result<()> {
        let Self {
            source: ShaderSource::SpirV(words),
            ..
        } = self;
        if words.is_empty() {
            return Err(Error::invalid_descriptor(
                "shader module",
                "source must not be empty",
            ));
        }
        Ok(())
    }
}

/// Kind of resource exposed by one bind-group binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BindingType {
    /// Uniform buffer.
    UniformBuffer,
    /// Storage buffer.
    StorageBuffer {
        /// Whether shaders may only read the buffer.
        read_only: bool,
    },
    /// Sampled image view.
    SampledImage,
    /// Storage image view.
    StorageImage {
        /// Whether shaders may only read the image.
        read_only: bool,
    },
    /// Sampler.
    Sampler,
    /// A sampled image view and sampler occupying one binding.
    CombinedImageSampler,
}

/// One binding declared by a bind-group layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BindGroupLayoutEntry {
    /// Binding number visible to shaders.
    pub binding: u32,
    /// Shader stages that can access the binding.
    pub visibility: ShaderStages,
    /// Resource kind at this binding.
    pub binding_type: BindingType,
    /// Number of resources in the binding array.
    pub count: u32,
}

/// Description of a bind-group layout.
#[derive(Clone, Copy, Debug)]
pub struct BindGroupLayoutCreateInfo<'a> {
    /// Declared bindings.
    pub entries: &'a [BindGroupLayoutEntry],
    /// Optional diagnostic label.
    pub label: Option<&'a str>,
}

impl BindGroupLayoutCreateInfo<'_> {
    /// Validates binding uniqueness and basic binding values.
    pub fn validate(&self) -> Result<()> {
        for (index, entry) in self.entries.iter().enumerate() {
            if entry.visibility.is_empty() {
                return Err(Error::invalid_descriptor(
                    "bind group layout",
                    "binding visibility must not be empty",
                ));
            }
            if entry.count == 0 {
                return Err(Error::invalid_descriptor(
                    "bind group layout",
                    "binding count must be non-zero",
                ));
            }
            if self.entries[..index]
                .iter()
                .any(|previous| previous.binding == entry.binding)
            {
                return Err(Error::invalid_descriptor(
                    "bind group layout",
                    "binding numbers must be unique",
                ));
            }
        }
        Ok(())
    }
}

/// Primitive assembly topology.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PrimitiveTopology {
    /// Independent triangle primitives.
    #[default]
    TriangleList,
    /// Connected triangle strip.
    TriangleStrip,
    /// Independent line primitives.
    LineList,
    /// Connected line strip.
    LineStrip,
    /// Point primitives.
    PointList,
}

/// Polygon face culling mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CullMode {
    /// Do not cull polygons.
    None,
    /// Cull front-facing polygons.
    Front,
    /// Cull back-facing polygons.
    #[default]
    Back,
}

/// Vertex winding considered front-facing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FrontFace {
    /// Clockwise winding.
    Clockwise,
    /// Counter-clockwise winding.
    #[default]
    CounterClockwise,
}

/// Depth and stencil comparison operation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CompareOperation {
    /// Never pass.
    Never,
    /// Pass when less.
    #[default]
    Less,
    /// Pass when equal.
    Equal,
    /// Pass when less or equal.
    LessEqual,
    /// Pass when greater.
    Greater,
    /// Pass when not equal.
    NotEqual,
    /// Pass when greater or equal.
    GreaterEqual,
    /// Always pass.
    Always,
}

/// Format of one vertex attribute.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VertexFormat {
    /// Two 32-bit floating-point values.
    Float32x2,
    /// Three 32-bit floating-point values.
    Float32x3,
    /// Four 32-bit floating-point values.
    Float32x4,
    /// One unsigned 32-bit integer.
    Uint32,
}

/// Rate at which a vertex buffer advances.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum VertexStepMode {
    /// Advance for every vertex.
    #[default]
    Vertex,
    /// Advance for every instance.
    Instance,
}

/// One shader input read from a vertex buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VertexAttribute {
    /// Byte offset from the start of the vertex.
    pub offset: u64,
    /// Shader location.
    pub location: u32,
    /// Attribute format.
    pub format: VertexFormat,
}

/// Layout of one bound vertex buffer.
#[derive(Clone, Copy, Debug)]
pub struct VertexBufferLayout<'a> {
    /// Byte stride between elements.
    pub stride: u64,
    /// Element stepping mode.
    pub step_mode: VertexStepMode,
    /// Attributes sourced from the buffer.
    pub attributes: &'a [VertexAttribute],
}

/// Depth state used by a graphics pipeline.
#[derive(Clone, Copy, Debug)]
pub struct DepthState {
    /// Depth attachment format.
    pub format: Format,
    /// Whether depth tests are enabled.
    pub test_enabled: bool,
    /// Whether passing fragments write depth.
    pub write_enabled: bool,
    /// Comparison used by depth tests.
    pub compare: CompareOperation,
}

/// Color output state used by a graphics pipeline.
#[derive(Clone, Copy, Debug)]
pub struct ColorTargetState {
    /// Color attachment format.
    pub format: Format,
    /// Color channels written by the pipeline.
    pub write_mask: ColorWrites,
}

/// Rasterization state used by a graphics pipeline.
#[derive(Clone, Copy, Debug, Default)]
pub struct PrimitiveState {
    /// Primitive topology.
    pub topology: PrimitiveTopology,
    /// Face culling mode.
    pub cull_mode: CullMode,
    /// Front-face winding.
    pub front_face: FrontFace,
}

/// Logical image layout used by render-graph synchronization.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ImageLayout {
    /// Contents are undefined and may be discarded.
    #[default]
    Undefined,
    /// General read/write layout.
    General,
    /// Color rendering attachment.
    ColorAttachment,
    /// Depth or stencil rendering attachment.
    DepthStencilAttachment,
    /// Shader-readable image.
    ShaderReadOnly,
    /// Transfer source.
    TransferSource,
    /// Transfer destination.
    TransferDestination,
    /// Ready for display presentation.
    Present,
}

/// Synchronization state tracked for an image resource.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ResourceState {
    /// Required image layout.
    pub layout: ImageLayout,
    /// Pipeline stages that access the resource.
    pub stages: PipelineStages,
    /// Types of accesses performed in those stages.
    pub access: AccessTypes,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_descriptor_rejects_empty_usage() {
        let info = BufferCreateInfo {
            size: 16,
            usage: BufferUsage::empty(),
            memory: MemoryLocation::Device,
            label: None,
        };
        assert!(matches!(
            info.validate(),
            Err(Error::InvalidDescriptor { .. })
        ));
    }

    #[test]
    fn multisampled_image_rejects_mip_chain() {
        let info = ImageCreateInfo {
            dimension: ImageDimension::Two,
            extent: Extent3D::new(64, 64, 1),
            format: Format::Rgba8Unorm,
            usage: ImageUsage::COLOR_ATTACHMENT,
            memory: MemoryLocation::Device,
            mip_levels: 2,
            array_layers: 1,
            samples: SampleCount::Four,
            label: None,
        };
        assert!(matches!(
            info.validate(),
            Err(Error::InvalidDescriptor { .. })
        ));
    }

    #[test]
    fn bind_group_layout_rejects_duplicate_bindings() {
        let entry = BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::VERTEX,
            binding_type: BindingType::UniformBuffer,
            count: 1,
        };
        let info = BindGroupLayoutCreateInfo {
            entries: &[entry, entry],
            label: None,
        };
        assert!(matches!(
            info.validate(),
            Err(Error::InvalidDescriptor { .. })
        ));
    }

    #[test]
    fn mip_count_reaches_one_by_one_level() {
        assert_eq!(Extent2D::new(1, 1).max_mip_levels(), 1);
        assert_eq!(Extent2D::new(1024, 512).max_mip_levels(), 11);
    }
}
