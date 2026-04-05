use ash::{Device, vk};

/// Complete GPU model.
pub struct Model {
    pub name: String,
    pub primitives: Vec<Primitive>,
    pub textures: Vec<Texture>,
    pub materials: Vec<resource_manager::Material>,
}

impl Model {
    pub fn record_cmd(&self, device: &Device, cmd: vk::CommandBuffer) {
        // Assuming you've built a descriptor set layout with combined image samplers:
        for prim in &self.primitives {
            if let Some(mat_idx) = prim.material {
                // from GpuPrimitive
                let mat = &self.materials[mat_idx];

                if let Some(tex_idx) = mat.base_color_texture() {
                    let tex = &self.textures[*tex_idx];

                    let image_info = vk::DescriptorImageInfo::default()
                        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                        .image_view(tex.view)
                        .sampler(tex.sampler);

                    let write = vk::WriteDescriptorSet::default()
                        // TODO: setup descriptor sets for the textures
                        //.dst_set(descriptor_set)
                        .dst_binding(0)
                        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                        .image_info(std::slice::from_ref(&image_info));

                    unsafe { device.update_descriptor_sets(&[write], &[]) };
                }
            }

            unsafe {
                device.cmd_bind_vertex_buffers(cmd, 0, &[prim.vertex_buffer], &[0]);
                device.cmd_bind_index_buffer(cmd, prim.index_buffer, 0, vk::IndexType::UINT32);
                device.cmd_draw_indexed(cmd, prim.index_count, 1, 0, 0, 0);
            }
        }
    }
    pub fn destroy(&self, device: &Device) {
        for prim in &self.primitives {
            unsafe {
                device.destroy_buffer(prim.vertex_buffer, None);
                device.free_memory(prim.vertex_buffer_memory, None);
                device.destroy_buffer(prim.index_buffer, None);
                device.free_memory(prim.index_buffer_memory, None);
            }
        }
        for tex in &self.textures {
            tex.destroy(device);
        }
    }
}

/// All GPU-side handles for a single texture.
pub struct Texture {
    pub image: vk::Image,
    pub memory: vk::DeviceMemory,
    pub view: vk::ImageView,
    pub sampler: vk::Sampler,
    pub mip_levels: u32,
}

impl Texture {
    fn destroy(&self, device: &Device) {
        unsafe {
            device.destroy_sampler(self.sampler, None);
            device.destroy_image_view(self.view, None);
            device.destroy_image(self.image, None);
            device.free_memory(self.memory, None);
        }
    }
}

/// GPU-side handles for a single glTF primitive.
pub struct Primitive {
    pub vertex_buffer: vk::Buffer,
    pub vertex_buffer_memory: vk::DeviceMemory,
    pub index_buffer: vk::Buffer,
    pub index_buffer_memory: vk::DeviceMemory,
    pub index_count: u32,
    pub material: Option<usize>,
}
