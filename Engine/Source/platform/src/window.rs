use log::debug;
use winit::{
    dpi::PhysicalSize,
    keyboard::ModifiersState,
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window::{Theme, WindowId},
};

pub struct Window {
    window: Box<dyn winit::window::Window>,
    focused: bool,
    theme: Theme,
    /// If the window is completely hidden (minized or covered by another
    /// window)
    occluded: bool,
    modifiers: ModifiersState,
}

impl Window {
    pub fn new(window: Box<dyn winit::window::Window>) -> Self {
        Self {
            focused: false,
            theme: window.theme().unwrap_or(Theme::Dark),
            occluded: false,
            modifiers: Default::default(),
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
    pub fn size(&self) -> PhysicalSize<u32> {
        self.window.surface_size()
    }
    /// Update if window is focused. This only updates internal state, do
    /// not call if you want to focus the window;
    pub fn focused(&mut self, focused: bool) {
        self.focused = focused;
    }
    pub fn set_draw_theme(&mut self, theme: Theme) {
        self.theme = theme;
        self.window.request_redraw();
    }
    pub fn set_occluded(&mut self, occluded: bool) {
        self.occluded = occluded
    }
    pub fn set_modifiers(&mut self, modifiers: ModifiersState) {
        self.modifiers = modifiers
    }
    pub fn get_modifiers(&mut self) -> &ModifiersState {
        &self.modifiers
    }
}

impl HasWindowHandle for Window {
    fn window_handle(
        &self,
    ) -> Result<winit::raw_window_handle::WindowHandle<'_>, winit::raw_window_handle::HandleError>
    {
        self.window.window_handle()
    }
}

impl HasDisplayHandle for Window {
    fn display_handle(
        &self,
    ) -> Result<winit::raw_window_handle::DisplayHandle<'_>, winit::raw_window_handle::HandleError>
    {
        self.window.display_handle()
    }
}
