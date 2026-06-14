use std::marker::PhantomData;

use ash::vk;

use crate::resources::descriptors::layouts::SetLayout;

pub struct SceneSet;

impl SetLayout for SceneSet {
    const BINDINGS: &'static [vk::DescriptorSetLayoutBinding<'static>] =
        &[vk::DescriptorSetLayoutBinding {
            binding: 0,
            descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
            descriptor_count: 1,
            stage_flags: vk::ShaderStageFlags::VERTEX,
            p_immutable_samplers: ::core::ptr::null(),
            _marker: PhantomData,
        }];
}

pub struct ObjectSet;

impl SetLayout for ObjectSet {
    const BINDINGS: &'static [vk::DescriptorSetLayoutBinding<'static>] =
        &[vk::DescriptorSetLayoutBinding {
            binding: 0,
            descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
            descriptor_count: 1,
            stage_flags: vk::ShaderStageFlags::VERTEX,
            p_immutable_samplers: ::core::ptr::null(),
            _marker: PhantomData,
        }];
}

pub struct MaterialSet;

impl SetLayout for MaterialSet {
    const BINDINGS: &'static [vk::DescriptorSetLayoutBinding<'static>] =
        &[vk::DescriptorSetLayoutBinding {
            binding: 0,
            descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            descriptor_count: 1,
            stage_flags: vk::ShaderStageFlags::FRAGMENT,
            p_immutable_samplers: ::core::ptr::null(),
            _marker: PhantomData,
        }];
}
