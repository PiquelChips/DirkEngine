use ash::vk;

#[derive(Clone, Copy, Default)]
pub struct QueueFamilyIndices {
    graphics_family: Option<u32>,
    present_family: Option<u32>,
}

pub struct Queues {
    graphics: vk::Queue,
    present: vk::Queue,
}

pub struct RendererProperties {
    msaa_samples: vk::SampleCountFlags,
    anisotropy: bool,
    surface_format: vk::SurfaceFormatKHR,
    min_image_count: u32,
    queue_family_indices: QueueFamilyIndices,
    depth_format: vk::Format,
}
