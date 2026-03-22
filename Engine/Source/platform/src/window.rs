use log::debug;
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
    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        let (width, height) = (size.width, size.height);

        // TODO: update the surface size
        debug!("Update window size to {width}/{height}");

        self.window.request_redraw();
    }
    /// Update if window is focused. This only updates internal state, do
    /// not call if you want to focus the window;
    pub fn focused(&mut self, focused: bool) {
        self.focused = focused;
    }
}
