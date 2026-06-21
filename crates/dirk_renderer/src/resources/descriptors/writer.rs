//! Batched descriptor set writes.

use ash::vk;

use crate::resources::descriptors::layouts::SetLayout;

use super::set::DescriptorSet;

enum WriteOp {
    UniformBuffer {
        set: vk::DescriptorSet,
        binding: u32,
        buffer: vk::Buffer,
        offset: vk::DeviceSize,
        range: vk::DeviceSize,
    },
    SampledImage {
        set: vk::DescriptorSet,
        binding: u32,
        view: vk::ImageView,
        image_layout: vk::ImageLayout,
    },
    Sampler {
        set: vk::DescriptorSet,
        binding: u32,
        sampler: vk::Sampler,
    },
}

/// Collects descriptor writes and submits them in one Vulkan call.
pub struct DescriptorWriter<'dev> {
    device: &'dev ash::Device,
    ops: Vec<WriteOp>,
}

impl<'dev> DescriptorWriter<'dev> {
    /// Creates an empty writer.
    pub fn new(device: &'dev ash::Device) -> Self {
        Self {
            device,
            ops: Vec::new(),
        }
    }

    /// Adds a uniform-buffer descriptor write.
    pub fn uniform_buffer<L: SetLayout>(
        mut self,
        set: &DescriptorSet<L>,
        binding: u32,
        buffer: vk::Buffer,
        range: vk::DeviceSize,
    ) -> Self {
        debug_assert_binding::<L>(binding, vk::DescriptorType::UNIFORM_BUFFER);
        self.ops.push(WriteOp::UniformBuffer {
            set: set.raw(),
            binding,
            buffer,
            offset: 0,
            range,
        });
        self
    }

    /// Adds a sampled image descriptor write.
    pub fn sampled_image<L: SetLayout>(
        mut self,
        set: &DescriptorSet<L>,
        binding: u32,
        view: vk::ImageView,
    ) -> Self {
        debug_assert_binding::<L>(binding, vk::DescriptorType::SAMPLED_IMAGE);
        self.ops.push(WriteOp::SampledImage {
            set: set.raw(),
            binding,
            view,
            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        });
        self
    }

    /// Adds a sampler descriptor write.
    pub fn sampler<L: SetLayout>(
        mut self,
        set: &DescriptorSet<L>,
        binding: u32,
        sampler: vk::Sampler,
    ) -> Self {
        debug_assert_binding::<L>(binding, vk::DescriptorType::SAMPLER);
        self.ops.push(WriteOp::Sampler {
            set: set.raw(),
            binding,
            sampler,
        });
        self
    }

    /// Submits all pending writes.
    pub fn flush(self) {
        if self.ops.is_empty() {
            return;
        }

        let mut buffer_infos = Vec::new();
        let mut image_infos = Vec::new();

        for op in &self.ops {
            match op {
                WriteOp::UniformBuffer {
                    buffer,
                    offset,
                    range,
                    ..
                } => {
                    buffer_infos.push(
                        vk::DescriptorBufferInfo::default()
                            .buffer(*buffer)
                            .offset(*offset)
                            .range(*range),
                    );
                }
                WriteOp::SampledImage {
                    view, image_layout, ..
                } => {
                    image_infos.push(
                        vk::DescriptorImageInfo::default()
                            .image_layout(*image_layout)
                            .image_view(*view),
                    );
                }
                WriteOp::Sampler { sampler, .. } => {
                    image_infos.push(vk::DescriptorImageInfo::default().sampler(*sampler));
                }
            }
        }

        let mut buffer_index = 0usize;
        let mut image_index = 0usize;
        let writes = self
            .ops
            .iter()
            .map(|op| match op {
                WriteOp::UniformBuffer { set, binding, .. } => {
                    let write = vk::WriteDescriptorSet::default()
                        .dst_set(*set)
                        .dst_binding(*binding)
                        .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                        .buffer_info(&buffer_infos[buffer_index..=buffer_index]);
                    buffer_index += 1;
                    write
                }
                WriteOp::SampledImage { set, binding, .. } => {
                    let write = vk::WriteDescriptorSet::default()
                        .dst_set(*set)
                        .dst_binding(*binding)
                        .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                        .image_info(&image_infos[image_index..=image_index]);
                    image_index += 1;
                    write
                }
                WriteOp::Sampler { set, binding, .. } => {
                    let write = vk::WriteDescriptorSet::default()
                        .dst_set(*set)
                        .dst_binding(*binding)
                        .descriptor_type(vk::DescriptorType::SAMPLER)
                        .image_info(&image_infos[image_index..=image_index]);
                    image_index += 1;
                    write
                }
            })
            .collect::<Vec<_>>();

        unsafe {
            self.device.update_descriptor_sets(&writes, &[]);
        }
    }
}

fn debug_assert_binding<L: SetLayout>(binding: u32, descriptor_type: vk::DescriptorType) {
    if cfg!(debug_assertions) {
        let Some(layout_binding) = L::BINDINGS
            .iter()
            .find(|layout_binding| layout_binding.binding == binding)
        else {
            panic!("descriptor binding {binding} does not exist in layout");
        };

        debug_assert_eq!(
            layout_binding.descriptor_type, descriptor_type,
            "descriptor binding {binding} has incompatible descriptor type"
        );
        debug_assert!(
            layout_binding.descriptor_count >= 1,
            "descriptor binding {binding} must have at least one descriptor"
        );
    }
}
