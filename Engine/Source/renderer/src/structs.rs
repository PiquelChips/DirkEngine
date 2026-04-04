use ash::{Device, Instance, vk};

/// All GPU-side handles for a single texture.
pub struct Texture {
    pub image: vk::Image,
    pub memory: vk::DeviceMemory,
    pub view: vk::ImageView,
    pub sampler: vk::Sampler,
    pub mip_levels: u32,
}

/// Resolved GPU material — ready to bind as descriptor sets.
pub struct Material {
    pub base_color_texture: Option<usize>,
    pub metallic_roughness_texture: Option<usize>,
    pub normal_texture: Option<usize>,
    pub occlusion_texture: Option<usize>,
    pub emissive_texture: Option<usize>,
}

/// Complete GPU model.
pub struct Model {
    pub primitives: Vec<Primitive>,
    pub textures: Vec<Texture>,
    pub materials: Vec<resource_manager::Material>,
}

/// GPU-side handles for a single glTF primitive.
pub struct Primitive {
    pub vertex_buffer: vk::Buffer,
    pub vertex_buffer_memory: vk::DeviceMemory,
    pub index_buffer: vk::Buffer,
    pub index_buffer_memory: vk::DeviceMemory,
    pub index_count: u32,
}

pub struct ModelUploader<'a> {
    pub instance: &'a Instance,
    pub device: &'a Device,
    pub physical_device: vk::PhysicalDevice,
    pub command_pool: vk::CommandPool,
    pub transfer_queue: vk::Queue,
}
