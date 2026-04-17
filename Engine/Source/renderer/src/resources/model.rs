use ash::vk;

use crate::resources::{
    buffer::{IndexBuffer, VertexBuffer},
    device::{Garbage, RenderDevice},
    image::Image,
};

/// Complete GPU model.
#[allow(unused)]
pub struct Model {
    pub name: String,
    pub primitives: Vec<Primitive>,
    pub textures: Vec<Texture>,
    pub materials: Vec<resource_manager::Material>,
    /// One descriptor set per entry in `materials`.
    /// `vk::DescriptorSet::null()` if the material has no base-colour texture.
    pub material_sets: Vec<vk::DescriptorSet>,
}

pub struct Texture {
    pub device: RenderDevice,
    pub image: Image,
    pub sampler: vk::Sampler,
}

impl Drop for Texture {
    fn drop(&mut self) {
        self.device.destroy(Garbage::Sampler(self.sampler));
    }
}

pub struct Primitive {
    pub vertex_buffer: VertexBuffer,
    pub index_buffer: IndexBuffer,
    pub index_count: u32,
    pub material: Option<usize>,
}
