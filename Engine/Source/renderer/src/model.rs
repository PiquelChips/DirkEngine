use ash::{Device, vk};

/// Complete GPU model.
#[derive(Clone)]
pub struct Model {
    pub name: String,
    pub primitives: Vec<Primitive>,
    pub textures: Vec<Texture>,
    pub materials: Vec<resource_manager::Material>,
}

impl Model {
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
