use ash::vk;

pub struct Window {
    surface: vk::SurfaceKHR,
}

impl Window {
    pub fn new(surface: vk::SurfaceKHR) -> Self {
        Self { surface }
    }
}
