use ash::{Device, vk};

/// Complete GPU model.
#[derive(Clone)]
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
    pub fn destroy(&self, device: &Device) {
        for prim in &self.primitives {
            prim.destroy(device);
        }
        for tex in &self.textures {
            tex.destroy(device);
        }
    }
}

/// All GPU-side handles for a single texture.
#[derive(Clone)]
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
#[derive(Clone)]
pub struct Primitive {
    pub vertex_buffer: vk::Buffer,
    pub vertex_buffer_memory: vk::DeviceMemory,
    pub index_buffer: vk::Buffer,
    pub index_buffer_memory: vk::DeviceMemory,
    pub index_count: u32,
    pub material: Option<usize>,
}

impl Primitive {
    fn destroy(&self, device: &Device) {
        unsafe {
            device.destroy_buffer(self.vertex_buffer, None);
            device.free_memory(self.vertex_buffer_memory, None);
            device.destroy_buffer(self.index_buffer, None);
            device.free_memory(self.index_buffer_memory, None);
        }
    }
}
