use ash::vk;
use gpu_allocator::MemoryLocation as AllocatorMemoryLocation;

#[cfg(feature = "presentation")]
use crate::PresentMode;
use crate::{
    AccessTypes, AddressMode, BindingType, BufferUsage, ColorWrites, CompareOperation, CullMode,
    Filter, Format, FrontFace, ImageAspects, ImageDimension, ImageLayout, ImageSubresourceLayers,
    ImageSubresourceRange, ImageUsage, ImageViewDimension, IndexFormat, MemoryLocation,
    PipelineStages, PrimitiveTopology, SampleCount, ShaderStages, VertexFormat, VertexStepMode,
};

pub(super) fn buffer_usage(value: BufferUsage) -> vk::BufferUsageFlags {
    let mut flags = vk::BufferUsageFlags::empty();
    add_flag(
        &mut flags,
        value.contains(BufferUsage::TRANSFER_SRC),
        vk::BufferUsageFlags::TRANSFER_SRC,
    );
    add_flag(
        &mut flags,
        value.contains(BufferUsage::TRANSFER_DST),
        vk::BufferUsageFlags::TRANSFER_DST,
    );
    add_flag(
        &mut flags,
        value.contains(BufferUsage::UNIFORM),
        vk::BufferUsageFlags::UNIFORM_BUFFER,
    );
    add_flag(
        &mut flags,
        value.contains(BufferUsage::STORAGE),
        vk::BufferUsageFlags::STORAGE_BUFFER,
    );
    add_flag(
        &mut flags,
        value.contains(BufferUsage::VERTEX),
        vk::BufferUsageFlags::VERTEX_BUFFER,
    );
    add_flag(
        &mut flags,
        value.contains(BufferUsage::INDEX),
        vk::BufferUsageFlags::INDEX_BUFFER,
    );
    flags
}

pub(super) fn image_usage(value: ImageUsage) -> vk::ImageUsageFlags {
    let mut flags = vk::ImageUsageFlags::empty();
    add_flag(
        &mut flags,
        value.contains(ImageUsage::TRANSFER_SRC),
        vk::ImageUsageFlags::TRANSFER_SRC,
    );
    add_flag(
        &mut flags,
        value.contains(ImageUsage::TRANSFER_DST),
        vk::ImageUsageFlags::TRANSFER_DST,
    );
    add_flag(
        &mut flags,
        value.contains(ImageUsage::SAMPLED),
        vk::ImageUsageFlags::SAMPLED,
    );
    add_flag(
        &mut flags,
        value.contains(ImageUsage::STORAGE),
        vk::ImageUsageFlags::STORAGE,
    );
    add_flag(
        &mut flags,
        value.contains(ImageUsage::COLOR_ATTACHMENT),
        vk::ImageUsageFlags::COLOR_ATTACHMENT,
    );
    add_flag(
        &mut flags,
        value.contains(ImageUsage::DEPTH_STENCIL_ATTACHMENT),
        vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
    );
    add_flag(
        &mut flags,
        value.contains(ImageUsage::TRANSIENT_ATTACHMENT),
        vk::ImageUsageFlags::TRANSIENT_ATTACHMENT,
    );
    flags
}

pub(super) fn image_aspects(value: ImageAspects) -> vk::ImageAspectFlags {
    let mut flags = vk::ImageAspectFlags::empty();
    add_flag(
        &mut flags,
        value.contains(ImageAspects::COLOR),
        vk::ImageAspectFlags::COLOR,
    );
    add_flag(
        &mut flags,
        value.contains(ImageAspects::DEPTH),
        vk::ImageAspectFlags::DEPTH,
    );
    add_flag(
        &mut flags,
        value.contains(ImageAspects::STENCIL),
        vk::ImageAspectFlags::STENCIL,
    );
    flags
}

pub(super) const fn format(value: Format) -> vk::Format {
    match value {
        Format::Rgba8Unorm => vk::Format::R8G8B8A8_UNORM,
        Format::Rgba8Srgb => vk::Format::R8G8B8A8_SRGB,
        Format::Bgra8Unorm => vk::Format::B8G8R8A8_UNORM,
        Format::Bgra8Srgb => vk::Format::B8G8R8A8_SRGB,
        Format::Rg32Float => vk::Format::R32G32_SFLOAT,
        Format::Rgb32Float => vk::Format::R32G32B32_SFLOAT,
        Format::Rgba32Float => vk::Format::R32G32B32A32_SFLOAT,
        Format::Depth16Unorm => vk::Format::D16_UNORM,
        Format::Depth32Float => vk::Format::D32_SFLOAT,
        Format::Depth24UnormStencil8 => vk::Format::D24_UNORM_S8_UINT,
        Format::Depth32FloatStencil8 => vk::Format::D32_SFLOAT_S8_UINT,
    }
}

#[cfg(any(feature = "presentation", test))]
pub(super) fn format_from_vk(value: vk::Format) -> crate::Result<Format> {
    match value {
        vk::Format::R8G8B8A8_UNORM => Ok(Format::Rgba8Unorm),
        vk::Format::R8G8B8A8_SRGB => Ok(Format::Rgba8Srgb),
        vk::Format::B8G8R8A8_UNORM => Ok(Format::Bgra8Unorm),
        vk::Format::B8G8R8A8_SRGB => Ok(Format::Bgra8Srgb),
        vk::Format::R32G32_SFLOAT => Ok(Format::Rg32Float),
        vk::Format::R32G32B32_SFLOAT => Ok(Format::Rgb32Float),
        vk::Format::R32G32B32A32_SFLOAT => Ok(Format::Rgba32Float),
        vk::Format::D16_UNORM => Ok(Format::Depth16Unorm),
        vk::Format::D32_SFLOAT => Ok(Format::Depth32Float),
        vk::Format::D24_UNORM_S8_UINT => Ok(Format::Depth24UnormStencil8),
        vk::Format::D32_SFLOAT_S8_UINT => Ok(Format::Depth32FloatStencil8),
        _ => Err(super::unsupported(format!(
            "Vulkan format {value:?} is not represented by the RHI"
        ))),
    }
}

pub(super) const fn sample_count(value: SampleCount) -> vk::SampleCountFlags {
    match value {
        SampleCount::One => vk::SampleCountFlags::TYPE_1,
        SampleCount::Two => vk::SampleCountFlags::TYPE_2,
        SampleCount::Four => vk::SampleCountFlags::TYPE_4,
        SampleCount::Eight => vk::SampleCountFlags::TYPE_8,
        SampleCount::Sixteen => vk::SampleCountFlags::TYPE_16,
        SampleCount::ThirtyTwo => vk::SampleCountFlags::TYPE_32,
        SampleCount::SixtyFour => vk::SampleCountFlags::TYPE_64,
    }
}

pub(super) const fn image_type(value: ImageDimension) -> vk::ImageType {
    match value {
        ImageDimension::One => vk::ImageType::TYPE_1D,
        ImageDimension::Two => vk::ImageType::TYPE_2D,
        ImageDimension::Three => vk::ImageType::TYPE_3D,
    }
}

pub(super) const fn image_view_type(value: ImageViewDimension) -> vk::ImageViewType {
    match value {
        ImageViewDimension::One => vk::ImageViewType::TYPE_1D,
        ImageViewDimension::Two => vk::ImageViewType::TYPE_2D,
        ImageViewDimension::Three => vk::ImageViewType::TYPE_3D,
        ImageViewDimension::Cube => vk::ImageViewType::CUBE,
        ImageViewDimension::OneArray => vk::ImageViewType::TYPE_1D_ARRAY,
        ImageViewDimension::TwoArray => vk::ImageViewType::TYPE_2D_ARRAY,
        ImageViewDimension::CubeArray => vk::ImageViewType::CUBE_ARRAY,
    }
}

pub(super) const fn memory_location(value: MemoryLocation) -> AllocatorMemoryLocation {
    match value {
        MemoryLocation::Device => AllocatorMemoryLocation::GpuOnly,
        MemoryLocation::Upload => AllocatorMemoryLocation::CpuToGpu,
        MemoryLocation::Readback => AllocatorMemoryLocation::GpuToCpu,
    }
}

pub(super) const fn filter(value: Filter) -> vk::Filter {
    match value {
        Filter::Nearest => vk::Filter::NEAREST,
        Filter::Linear => vk::Filter::LINEAR,
    }
}

pub(super) const fn mipmap_mode(value: Filter) -> vk::SamplerMipmapMode {
    match value {
        Filter::Nearest => vk::SamplerMipmapMode::NEAREST,
        Filter::Linear => vk::SamplerMipmapMode::LINEAR,
    }
}

pub(super) const fn address_mode(value: AddressMode) -> vk::SamplerAddressMode {
    match value {
        AddressMode::Repeat => vk::SamplerAddressMode::REPEAT,
        AddressMode::MirrorRepeat => vk::SamplerAddressMode::MIRRORED_REPEAT,
        AddressMode::ClampToEdge => vk::SamplerAddressMode::CLAMP_TO_EDGE,
    }
}

pub(super) fn shader_stages(value: ShaderStages) -> vk::ShaderStageFlags {
    let mut flags = vk::ShaderStageFlags::empty();
    add_flag(
        &mut flags,
        value.contains(ShaderStages::VERTEX),
        vk::ShaderStageFlags::VERTEX,
    );
    add_flag(
        &mut flags,
        value.contains(ShaderStages::FRAGMENT),
        vk::ShaderStageFlags::FRAGMENT,
    );
    add_flag(
        &mut flags,
        value.contains(ShaderStages::COMPUTE),
        vk::ShaderStageFlags::COMPUTE,
    );
    flags
}

pub(super) const fn descriptor_type(value: BindingType) -> vk::DescriptorType {
    match value {
        BindingType::UniformBuffer => vk::DescriptorType::UNIFORM_BUFFER,
        BindingType::StorageBuffer { .. } => vk::DescriptorType::STORAGE_BUFFER,
        BindingType::SampledImage => vk::DescriptorType::SAMPLED_IMAGE,
        BindingType::StorageImage { .. } => vk::DescriptorType::STORAGE_IMAGE,
        BindingType::Sampler => vk::DescriptorType::SAMPLER,
        BindingType::CombinedImageSampler => vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
    }
}

pub(super) const fn primitive_topology(value: PrimitiveTopology) -> vk::PrimitiveTopology {
    match value {
        PrimitiveTopology::TriangleList => vk::PrimitiveTopology::TRIANGLE_LIST,
        PrimitiveTopology::TriangleStrip => vk::PrimitiveTopology::TRIANGLE_STRIP,
        PrimitiveTopology::LineList => vk::PrimitiveTopology::LINE_LIST,
        PrimitiveTopology::LineStrip => vk::PrimitiveTopology::LINE_STRIP,
        PrimitiveTopology::PointList => vk::PrimitiveTopology::POINT_LIST,
    }
}

pub(super) const fn cull_mode(value: CullMode) -> vk::CullModeFlags {
    match value {
        CullMode::None => vk::CullModeFlags::NONE,
        CullMode::Front => vk::CullModeFlags::FRONT,
        CullMode::Back => vk::CullModeFlags::BACK,
    }
}

pub(super) const fn front_face(value: FrontFace) -> vk::FrontFace {
    match value {
        FrontFace::Clockwise => vk::FrontFace::CLOCKWISE,
        FrontFace::CounterClockwise => vk::FrontFace::COUNTER_CLOCKWISE,
    }
}

pub(super) const fn compare_op(value: CompareOperation) -> vk::CompareOp {
    match value {
        CompareOperation::Never => vk::CompareOp::NEVER,
        CompareOperation::Less => vk::CompareOp::LESS,
        CompareOperation::Equal => vk::CompareOp::EQUAL,
        CompareOperation::LessEqual => vk::CompareOp::LESS_OR_EQUAL,
        CompareOperation::Greater => vk::CompareOp::GREATER,
        CompareOperation::NotEqual => vk::CompareOp::NOT_EQUAL,
        CompareOperation::GreaterEqual => vk::CompareOp::GREATER_OR_EQUAL,
        CompareOperation::Always => vk::CompareOp::ALWAYS,
    }
}

pub(super) const fn vertex_format(value: VertexFormat) -> vk::Format {
    match value {
        VertexFormat::Float32x2 => vk::Format::R32G32_SFLOAT,
        VertexFormat::Float32x3 => vk::Format::R32G32B32_SFLOAT,
        VertexFormat::Float32x4 => vk::Format::R32G32B32A32_SFLOAT,
        VertexFormat::Uint32 => vk::Format::R32_UINT,
    }
}

pub(super) const fn vertex_rate(value: VertexStepMode) -> vk::VertexInputRate {
    match value {
        VertexStepMode::Vertex => vk::VertexInputRate::VERTEX,
        VertexStepMode::Instance => vk::VertexInputRate::INSTANCE,
    }
}

pub(super) fn color_writes(value: ColorWrites) -> vk::ColorComponentFlags {
    let mut flags = vk::ColorComponentFlags::empty();
    add_flag(
        &mut flags,
        value.contains(ColorWrites::RED),
        vk::ColorComponentFlags::R,
    );
    add_flag(
        &mut flags,
        value.contains(ColorWrites::GREEN),
        vk::ColorComponentFlags::G,
    );
    add_flag(
        &mut flags,
        value.contains(ColorWrites::BLUE),
        vk::ColorComponentFlags::B,
    );
    add_flag(
        &mut flags,
        value.contains(ColorWrites::ALPHA),
        vk::ColorComponentFlags::A,
    );
    flags
}

pub(super) const fn image_layout(value: ImageLayout) -> vk::ImageLayout {
    match value {
        ImageLayout::Undefined => vk::ImageLayout::UNDEFINED,
        ImageLayout::General => vk::ImageLayout::GENERAL,
        ImageLayout::ColorAttachment => vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        ImageLayout::DepthStencilAttachment => vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        ImageLayout::ShaderReadOnly => vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        ImageLayout::TransferSource => vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
        ImageLayout::TransferDestination => vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        ImageLayout::Present => vk::ImageLayout::PRESENT_SRC_KHR,
    }
}

pub(super) fn pipeline_stages(value: PipelineStages) -> vk::PipelineStageFlags2 {
    let mut flags = vk::PipelineStageFlags2::empty();
    add_flag(
        &mut flags,
        value.contains(PipelineStages::TOP),
        vk::PipelineStageFlags2::TOP_OF_PIPE,
    );
    add_flag(
        &mut flags,
        value.contains(PipelineStages::VERTEX_SHADER),
        vk::PipelineStageFlags2::VERTEX_SHADER,
    );
    add_flag(
        &mut flags,
        value.contains(PipelineStages::FRAGMENT_SHADER),
        vk::PipelineStageFlags2::FRAGMENT_SHADER,
    );
    add_flag(
        &mut flags,
        value.contains(PipelineStages::COMPUTE_SHADER),
        vk::PipelineStageFlags2::COMPUTE_SHADER,
    );
    add_flag(
        &mut flags,
        value.contains(PipelineStages::EARLY_DEPTH_STENCIL),
        vk::PipelineStageFlags2::EARLY_FRAGMENT_TESTS,
    );
    add_flag(
        &mut flags,
        value.contains(PipelineStages::LATE_DEPTH_STENCIL),
        vk::PipelineStageFlags2::LATE_FRAGMENT_TESTS,
    );
    add_flag(
        &mut flags,
        value.contains(PipelineStages::COLOR_OUTPUT),
        vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
    );
    add_flag(
        &mut flags,
        value.contains(PipelineStages::TRANSFER),
        vk::PipelineStageFlags2::TRANSFER,
    );
    add_flag(
        &mut flags,
        value.contains(PipelineStages::BOTTOM),
        vk::PipelineStageFlags2::BOTTOM_OF_PIPE,
    );
    add_flag(
        &mut flags,
        value.contains(PipelineStages::ALL_COMMANDS),
        vk::PipelineStageFlags2::ALL_COMMANDS,
    );
    flags
}

pub(super) fn access_types(value: AccessTypes) -> vk::AccessFlags2 {
    let mut flags = vk::AccessFlags2::empty();
    add_flag(
        &mut flags,
        value.contains(AccessTypes::UNIFORM_READ),
        vk::AccessFlags2::UNIFORM_READ,
    );
    add_flag(
        &mut flags,
        value.contains(AccessTypes::SHADER_READ),
        vk::AccessFlags2::SHADER_READ,
    );
    add_flag(
        &mut flags,
        value.contains(AccessTypes::SHADER_WRITE),
        vk::AccessFlags2::SHADER_WRITE,
    );
    add_flag(
        &mut flags,
        value.contains(AccessTypes::COLOR_ATTACHMENT_READ),
        vk::AccessFlags2::COLOR_ATTACHMENT_READ,
    );
    add_flag(
        &mut flags,
        value.contains(AccessTypes::COLOR_ATTACHMENT_WRITE),
        vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
    );
    add_flag(
        &mut flags,
        value.contains(AccessTypes::DEPTH_STENCIL_READ),
        vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_READ,
    );
    add_flag(
        &mut flags,
        value.contains(AccessTypes::DEPTH_STENCIL_WRITE),
        vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE,
    );
    add_flag(
        &mut flags,
        value.contains(AccessTypes::TRANSFER_READ),
        vk::AccessFlags2::TRANSFER_READ,
    );
    add_flag(
        &mut flags,
        value.contains(AccessTypes::TRANSFER_WRITE),
        vk::AccessFlags2::TRANSFER_WRITE,
    );
    add_flag(
        &mut flags,
        value.contains(AccessTypes::HOST_READ),
        vk::AccessFlags2::HOST_READ,
    );
    add_flag(
        &mut flags,
        value.contains(AccessTypes::HOST_WRITE),
        vk::AccessFlags2::HOST_WRITE,
    );
    add_flag(
        &mut flags,
        value.contains(AccessTypes::MEMORY_READ),
        vk::AccessFlags2::MEMORY_READ,
    );
    add_flag(
        &mut flags,
        value.contains(AccessTypes::MEMORY_WRITE),
        vk::AccessFlags2::MEMORY_WRITE,
    );
    flags
}

pub(super) const fn index_type(value: IndexFormat) -> vk::IndexType {
    match value {
        IndexFormat::Uint16 => vk::IndexType::UINT16,
        IndexFormat::Uint32 => vk::IndexType::UINT32,
    }
}

#[cfg(feature = "presentation")]
pub(super) const fn present_mode(value: PresentMode) -> vk::PresentModeKHR {
    match value {
        PresentMode::Fifo => vk::PresentModeKHR::FIFO,
        PresentMode::Mailbox => vk::PresentModeKHR::MAILBOX,
        PresentMode::Immediate => vk::PresentModeKHR::IMMEDIATE,
    }
}

pub(super) fn subresource_range(value: ImageSubresourceRange) -> vk::ImageSubresourceRange {
    vk::ImageSubresourceRange {
        aspect_mask: image_aspects(value.aspects),
        base_mip_level: value.base_mip_level,
        level_count: value.mip_level_count,
        base_array_layer: value.base_array_layer,
        layer_count: value.array_layer_count,
    }
}

pub(super) fn subresource_layers(value: ImageSubresourceLayers) -> vk::ImageSubresourceLayers {
    vk::ImageSubresourceLayers {
        aspect_mask: image_aspects(value.aspects),
        mip_level: value.mip_level,
        base_array_layer: value.base_array_layer,
        layer_count: value.array_layer_count,
    }
}

fn add_flag<T>(flags: &mut T, condition: bool, flag: T)
where
    T: Copy + std::ops::BitOrAssign,
{
    if condition {
        *flags |= flag;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_usage_preserves_every_selected_flag() {
        let value = BufferUsage::TRANSFER_SRC | BufferUsage::VERTEX | BufferUsage::INDEX;
        assert_eq!(
            buffer_usage(value),
            vk::BufferUsageFlags::TRANSFER_SRC
                | vk::BufferUsageFlags::VERTEX_BUFFER
                | vk::BufferUsageFlags::INDEX_BUFFER
        );
    }

    #[test]
    fn resource_state_mappings_preserve_combined_hazards() {
        let stages = PipelineStages::EARLY_DEPTH_STENCIL | PipelineStages::LATE_DEPTH_STENCIL;
        let access = AccessTypes::DEPTH_STENCIL_READ | AccessTypes::DEPTH_STENCIL_WRITE;
        assert_eq!(
            pipeline_stages(stages),
            vk::PipelineStageFlags2::EARLY_FRAGMENT_TESTS
                | vk::PipelineStageFlags2::LATE_FRAGMENT_TESTS
        );
        assert_eq!(
            access_types(access),
            vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_READ
                | vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE
        );
    }

    #[test]
    fn supported_formats_round_trip() {
        let formats = [
            Format::Rgba8Unorm,
            Format::Rgba8Srgb,
            Format::Bgra8Unorm,
            Format::Bgra8Srgb,
            Format::Depth32Float,
            Format::Depth24UnormStencil8,
        ];
        for value in formats {
            assert_eq!(
                format_from_vk(format(value)).expect("format should map"),
                value
            );
        }
    }
}
