//! Compile-time marker types for descriptor set layouts.

use ash::vk;

use super::{DescriptorLayouts, slots};

mod sealed {
    pub trait Sealed {}
}

/// Associates a Rust marker type with a specific Vulkan descriptor set layout.
pub trait SetLayout: sealed::Sealed + Send + Sync + 'static {
    /// Pipeline set index (`set = N` in GLSL).
    #[allow(dead_code)]
    const SET_SLOT: u32;
    /// Pipeline set index as a Rust array index.
    const SET_INDEX: usize;
    /// Binding index within the set (`binding = M` in GLSL).
    const BINDING: u32;
    /// Shader stages that access this set.
    const STAGE: vk::ShaderStageFlags;
    /// Vulkan descriptor type for the single binding in this set.
    const DESCRIPTOR_TYPE: vk::DescriptorType;
    /// Number of descriptors in the single binding.
    const DESCRIPTORS_PER_SET: u32;

    /// Retrieves the pre-built layout handle from the global layout registry.
    fn layout(layouts: &DescriptorLayouts) -> vk::DescriptorSetLayout;
}

/// Layouts whose single binding is a uniform buffer.
pub trait UniformBufferLayout: SetLayout {}

/// Layouts whose single binding is a combined image sampler.
pub trait CombinedImageSamplerLayout: SetLayout {}

/// Marker for the per-scene descriptor set.
pub struct SceneLayout;
/// Marker for the per-object descriptor set.
pub struct ObjectLayout;
/// Marker for the per-material descriptor set.
pub struct MaterialLayout;

impl sealed::Sealed for SceneLayout {}
impl sealed::Sealed for ObjectLayout {}
impl sealed::Sealed for MaterialLayout {}

impl SetLayout for SceneLayout {
    const SET_SLOT: u32 = slots::SCENE_SET;
    const SET_INDEX: usize = slots::SCENE_SET_INDEX;
    const BINDING: u32 = slots::SCENE_BINDING;
    const STAGE: vk::ShaderStageFlags = slots::SCENE_STAGE;
    const DESCRIPTOR_TYPE: vk::DescriptorType = vk::DescriptorType::UNIFORM_BUFFER;
    const DESCRIPTORS_PER_SET: u32 = 1;

    fn layout(layouts: &DescriptorLayouts) -> vk::DescriptorSetLayout {
        layouts.scene
    }
}

impl SetLayout for ObjectLayout {
    const SET_SLOT: u32 = slots::OBJECT_SET;
    const SET_INDEX: usize = slots::OBJECT_SET_INDEX;
    const BINDING: u32 = slots::OBJECT_BINDING;
    const STAGE: vk::ShaderStageFlags = slots::OBJECT_STAGE;
    const DESCRIPTOR_TYPE: vk::DescriptorType = vk::DescriptorType::UNIFORM_BUFFER;
    const DESCRIPTORS_PER_SET: u32 = 1;

    fn layout(layouts: &DescriptorLayouts) -> vk::DescriptorSetLayout {
        layouts.object
    }
}

impl SetLayout for MaterialLayout {
    const SET_SLOT: u32 = slots::MATERIAL_SET;
    const SET_INDEX: usize = slots::MATERIAL_SET_INDEX;
    const BINDING: u32 = slots::MATERIAL_BINDING;
    const STAGE: vk::ShaderStageFlags = slots::MATERIAL_STAGE;
    const DESCRIPTOR_TYPE: vk::DescriptorType = vk::DescriptorType::COMBINED_IMAGE_SAMPLER;
    const DESCRIPTORS_PER_SET: u32 = 1;

    fn layout(layouts: &DescriptorLayouts) -> vk::DescriptorSetLayout {
        layouts.material
    }
}

impl UniformBufferLayout for SceneLayout {}
impl UniformBufferLayout for ObjectLayout {}
impl CombinedImageSamplerLayout for MaterialLayout {}
