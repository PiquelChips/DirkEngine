use ash::{Device, vk};
use gpu_allocator::vulkan::{Allocation, Allocator};

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

impl Model {
    pub fn destroy(&mut self, device: &Device, allocator: &mut Allocator) {
        for prim in &mut self.primitives {
            prim.destroy(device, allocator);
        }
        for tex in &mut self.textures {
            tex.destroy(device, allocator);
        }
    }
}

/// All GPU-side handles for a single texture.
pub struct Texture {
    pub image: vk::Image,
    pub alloc: Option<Allocation>,
    pub view: vk::ImageView,
    pub sampler: vk::Sampler,
    pub mip_levels: u32,
}

impl Texture {
    fn destroy(&mut self, device: &Device, allocator: &mut Allocator) {
        if let Some(alloc) = self.alloc.take() {
            allocator.free(alloc).unwrap();
        }

        unsafe {
            device.destroy_sampler(self.sampler, None);
            device.destroy_image_view(self.view, None);
            device.destroy_image(self.image, None);
        }
    }
}

/// GPU-side handles for a single glTF primitive.
pub struct Primitive {
    pub vertex_buffer: vk::Buffer,
    pub vertex_buffer_alloc: Option<Allocation>,
    pub index_buffer: vk::Buffer,
    pub index_buffer_alloc: Option<Allocation>,
    pub index_count: u32,
    pub material: Option<usize>,
}

impl Primitive {
    fn destroy(&mut self, device: &Device, allocator: &mut Allocator) {
        if let Some(alloc) = self.vertex_buffer_alloc.take() {
            allocator.free(alloc).unwrap();
        }
        if let Some(alloc) = self.index_buffer_alloc.take() {
            allocator.free(alloc).unwrap();
        }
        unsafe {
            device.destroy_buffer(self.vertex_buffer, None);
            device.destroy_buffer(self.index_buffer, None);
        }
    }
}
