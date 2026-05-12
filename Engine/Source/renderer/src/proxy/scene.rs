use std::collections::HashSet;

use ash::vk;
use gpu_allocator::MemoryLocation;
use universe::{Entity, WorldId};

use crate::{
    MAX_FRAMES_IN_FLIGHT, Result,
    proxy::{SceneManager, SceneUbo},
    resources::buffer::UniformBuffer,
};

/// Renderer representation of a [`World`].
pub struct Scene {
    world: WorldId,
    entities: HashSet<Entity>,

    ubo: [UniformBuffer; MAX_FRAMES_IN_FLIGHT],
    descriptor_sets: [vk::DescriptorSet; MAX_FRAMES_IN_FLIGHT],
}

impl Scene {
    /// Builds a [Scene].
    /// Constructs the renderer stuff like command pools, descriptor sets, ... from
    /// the [Renderer].
    pub fn build(manager: &SceneManager, world: WorldId) -> Result<Self> {
        // Allocate scene-level sets (one per frame)
        let layouts = [manager.device.layouts.scene; MAX_FRAMES_IN_FLIGHT];
        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(manager.descriptor_pool)
            .set_layouts(&layouts);

        let scene_desc_sets: [vk::DescriptorSet; MAX_FRAMES_IN_FLIGHT] = unsafe {
            manager
                .device
                .device
                .allocate_descriptor_sets(&alloc_info)?
                .try_into()
                .expect("should be able to convert desc_sets to array")
        };

        let ubo_size = size_of::<SceneUbo>() as u64;
        let build_ubo =
            || UniformBuffer::create(&manager.device, ubo_size, MemoryLocation::CpuToGpu);
        let ubo = [build_ubo()?, build_ubo()?];

        let buffer_infos: [vk::DescriptorBufferInfo; MAX_FRAMES_IN_FLIGHT] =
            std::array::from_fn(|i| {
                vk::DescriptorBufferInfo::default()
                    .buffer(ubo[i].buffer())
                    .range(size_of::<SceneUbo>() as u64)
                    .offset(0)
            });

        let descriptor_writes: [vk::WriteDescriptorSet; MAX_FRAMES_IN_FLIGHT] =
            std::array::from_fn(|i| {
                vk::WriteDescriptorSet::default()
                    .dst_set(scene_desc_sets[i])
                    .dst_binding(0)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .buffer_info(std::slice::from_ref(&buffer_infos[i]))
            });

        unsafe {
            manager
                .device
                .device
                .update_descriptor_sets(&descriptor_writes, &[]);
        };

        Ok(Self {
            world,
            entities: HashSet::new(),
            ubo,
            descriptor_sets: scene_desc_sets,
        })
    }
}
