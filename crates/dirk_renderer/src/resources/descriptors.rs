//! Safe descriptor layout and descriptor pool helpers.

use ash::{Device, prelude::VkResult, vk};

use crate::{
    Error, Result,
    resources::device::{Garbage, RenderDevice},
};

/// All descriptor set layouts used by the renderer.
pub struct DescriptorLayouts {
    /// Per scene layout. Holds view and projection matrices for rendering.
    pub scene: vk::DescriptorSetLayout,
    /// Per object layout. Holds the model matrix.
    pub object: vk::DescriptorSetLayout,
    /// Per material layout. Holds a base color texture descriptor.
    pub material: vk::DescriptorSetLayout,
}

impl DescriptorLayouts {
    /// Creates the descriptor set layouts used by the renderer.
    pub fn create(device: &Device) -> Result<Self> {
        Ok(Self {
            scene: create_layout(
                device,
                0,
                vk::DescriptorType::UNIFORM_BUFFER,
                vk::ShaderStageFlags::VERTEX,
            )?,
            object: create_layout(
                device,
                1,
                vk::DescriptorType::UNIFORM_BUFFER,
                vk::ShaderStageFlags::VERTEX,
            )?,
            material: create_layout(
                device,
                2,
                vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                vk::ShaderStageFlags::FRAGMENT,
            )?,
        })
    }

    /// Returns the layouts in pipeline set order.
    pub fn pipeline_layouts(&self) -> [vk::DescriptorSetLayout; 3] {
        [self.scene, self.object, self.material]
    }

    /// Destroys every descriptor set layout.
    pub fn destroy(&self, device: &Device) {
        unsafe {
            device.destroy_descriptor_set_layout(self.scene, None);
            device.destroy_descriptor_set_layout(self.object, None);
            device.destroy_descriptor_set_layout(self.material, None);
        }
    }
}

fn create_layout(
    device: &Device,
    binding: u32,
    descriptor_type: vk::DescriptorType,
    stage_flags: vk::ShaderStageFlags,
) -> Result<vk::DescriptorSetLayout> {
    let binding = vk::DescriptorSetLayoutBinding::default()
        .binding(binding)
        .descriptor_type(descriptor_type)
        .descriptor_count(1)
        .stage_flags(stage_flags);

    let info =
        vk::DescriptorSetLayoutCreateInfo::default().bindings(std::slice::from_ref(&binding));

    Ok(unsafe { device.create_descriptor_set_layout(&info, None)? })
}

/// A non-owning descriptor set handle allocated from a [`DescriptorAllocator`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DescriptorSet {
    raw: vk::DescriptorSet,
}

impl DescriptorSet {
    /// Creates a null descriptor set handle.
    pub fn null() -> Self {
        Self {
            raw: vk::DescriptorSet::null(),
        }
    }

    /// Returns the raw Vulkan descriptor set handle.
    pub fn raw(self) -> vk::DescriptorSet {
        self.raw
    }
}

/// Descriptor count to reserve per descriptor set in each pool page.
#[derive(Clone, Copy, Debug)]
pub struct DescriptorPoolSize {
    ty: vk::DescriptorType,
    descriptors_per_set: u32,
}

impl DescriptorPoolSize {
    /// Creates a descriptor pool size rule.
    pub const fn new(ty: vk::DescriptorType, descriptors_per_set: u32) -> Self {
        Self {
            ty,
            descriptors_per_set,
        }
    }
}

struct DescriptorPoolPage {
    raw: vk::DescriptorPool,
}

/// Growable descriptor pool allocator.
///
/// Vulkan descriptor pools are fixed-size, so this allocator creates more pool
/// pages when existing pages can no longer satisfy an allocation.
pub struct DescriptorAllocator {
    device: RenderDevice,
    pool_sizes: Vec<DescriptorPoolSize>,
    flags: vk::DescriptorPoolCreateFlags,
    pools: Vec<DescriptorPoolPage>,
    next_max_sets: u32,
}

impl DescriptorAllocator {
    /// Creates a growable descriptor allocator.
    pub fn new(
        device: &RenderDevice,
        pool_sizes: &[DescriptorPoolSize],
        initial_max_sets: u32,
    ) -> Result<Self> {
        let initial_max_sets = initial_max_sets.max(1);
        let mut allocator = Self {
            device: device.clone(),
            pool_sizes: pool_sizes.to_vec(),
            flags: vk::DescriptorPoolCreateFlags::empty(),
            pools: Vec::new(),
            next_max_sets: initial_max_sets,
        };
        allocator.add_pool(initial_max_sets)?;
        Ok(allocator)
    }

    /// Allocates `count` descriptor sets with the same layout.
    pub fn allocate_many(
        &mut self,
        layout: vk::DescriptorSetLayout,
        count: usize,
    ) -> Result<Vec<DescriptorSet>> {
        let layouts = vec![layout; count];
        self.allocate(&layouts)
    }

    /// Allocates a fixed-size array of descriptor sets with the same layout.
    pub fn allocate_array<const N: usize>(
        &mut self,
        layout: vk::DescriptorSetLayout,
    ) -> Result<[DescriptorSet; N]> {
        let layouts = [layout; N];
        let sets = self.allocate(&layouts)?;
        Ok(std::array::from_fn(|i| sets[i]))
    }

    /// Writes a uniform buffer descriptor into `set`.
    pub fn write_uniform_buffer(
        &self,
        set: DescriptorSet,
        binding: u32,
        buffer: vk::Buffer,
        range: vk::DeviceSize,
    ) {
        let buffer_info = vk::DescriptorBufferInfo::default()
            .buffer(buffer)
            .range(range)
            .offset(0);
        let write = vk::WriteDescriptorSet::default()
            .dst_set(set.raw)
            .dst_binding(binding)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .buffer_info(std::slice::from_ref(&buffer_info));

        unsafe {
            self.device.device.update_descriptor_sets(&[write], &[]);
        }
    }

    /// Writes a combined image sampler descriptor into `set`.
    pub fn write_combined_image_sampler(
        &self,
        set: DescriptorSet,
        binding: u32,
        image_view: vk::ImageView,
        sampler: vk::Sampler,
    ) {
        let image_info = vk::DescriptorImageInfo::default()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(image_view)
            .sampler(sampler);
        let write = vk::WriteDescriptorSet::default()
            .dst_set(set.raw)
            .dst_binding(binding)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(std::slice::from_ref(&image_info));

        unsafe {
            self.device.device.update_descriptor_sets(&[write], &[]);
        }
    }

    fn allocate(&mut self, layouts: &[vk::DescriptorSetLayout]) -> Result<Vec<DescriptorSet>> {
        if layouts.is_empty() {
            return Ok(Vec::new());
        }

        let required_sets = u32::try_from(layouts.len())
            .map_err(|_| Error::DescriptorSetCountTooLarge(layouts.len()))?;

        for pool in &self.pools {
            match self.allocate_from_pool(pool.raw, layouts) {
                Ok(sets) => return Ok(sets), // returns sets when pool is found
                Err(vk::Result::ERROR_OUT_OF_POOL_MEMORY | vk::Result::ERROR_FRAGMENTED_POOL) => {}
                Err(error) => return Err(error.into()),
            }
        }

        // if no pool found, create one
        let pool = self.add_pool(required_sets)?;
        Ok(self.allocate_from_pool(pool, layouts)?)
    }

    fn allocate_from_pool(
        &self,
        pool: vk::DescriptorPool,
        layouts: &[vk::DescriptorSetLayout],
    ) -> VkResult<Vec<DescriptorSet>> {
        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(pool)
            .set_layouts(layouts);

        unsafe { self.device.device.allocate_descriptor_sets(&alloc_info) }.map(|sets| {
            sets.into_iter()
                .map(|raw| DescriptorSet { raw })
                .collect::<Vec<_>>()
        })
    }

    fn add_pool(&mut self, required_sets: u32) -> Result<vk::DescriptorPool> {
        let max_sets = self.next_max_sets.max(required_sets).max(1);
        self.next_max_sets = max_sets.saturating_mul(2).max(1);
        let pool = self.create_pool(max_sets)?;
        self.pools.push(DescriptorPoolPage { raw: pool });
        Ok(pool)
    }

    fn create_pool(&self, max_sets: u32) -> Result<vk::DescriptorPool> {
        let pool_sizes = self
            .pool_sizes
            .iter()
            .map(|size| {
                vk::DescriptorPoolSize::default()
                    .ty(size.ty)
                    .descriptor_count(size.descriptors_per_set.saturating_mul(max_sets).max(1))
            })
            .collect::<Vec<_>>();

        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .flags(self.flags)
            .pool_sizes(&pool_sizes)
            .max_sets(max_sets);

        Ok(unsafe {
            self.device
                .device
                .create_descriptor_pool(&pool_info, None)?
        })
    }
}

impl Drop for DescriptorAllocator {
    fn drop(&mut self) {
        for pool in self.pools.drain(..) {
            self.device.destroy(Garbage::DescriptorPool(pool.raw));
        }
    }
}
