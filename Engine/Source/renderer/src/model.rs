use ash::{Device, vk};

use crate::{
    buffer::{IndexBuffer, VertexBuffer},
    image::Image,
};

/// Complete GPU model.
pub struct Model {
    pub name: String,
    pub primitives: Vec<Primitive>,
    pub textures: Vec<Texture>,
    pub materials: Vec<resource_manager::Material>,
    /// One descriptor set per entry in `materials`.
    /// `vk::DescriptorSet::null()` if the material has no base-colour texture.
    pub material_sets: Vec<vk::DescriptorSet>,
}

/// All GPU-side handles for a single texture.
pub struct Texture {
    pub device: Device,
    pub image: Image,
    pub sampler: vk::Sampler,
    pub mip_levels: u32,
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_sampler(self.sampler, None);
        }
    }
}

/// GPU-side handles for a single glTF primitive.
pub struct Primitive {
    pub vertex_buffer: VertexBuffer,
    pub index_buffer: IndexBuffer,
    pub index_count: u32,
    pub material: Option<usize>,
}
