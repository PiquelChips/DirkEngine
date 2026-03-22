use winit::{dpi::PhysicalSize, window::WindowId};

pub struct Window {
    focused: bool,
    window: Box<dyn winit::window::Window>,
}

impl Window {
    pub fn new(window: Box<dyn winit::window::Window>) -> Self {
        Self {
            focused: false,
            window,
        }
    }
    pub fn id(&self) -> WindowId {
        self.window.id()
    }
    pub fn resize(&mut self, _size: PhysicalSize<u32>) {
        todo!("Window::resize")
    }
    /// Update if window is focused. This only updates internal state, do
    /// not call if you want to focus the window;
    pub fn focused(&mut self, focused: bool) {
        self.focused = focused;
    }
}
