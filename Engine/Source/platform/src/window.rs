use winit::{
    dpi::PhysicalSize,
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window::{Theme, WindowId},
};

use crate::event::WindowEvent;

/// Internal platform representation of a window. Holds the
/// [`winit::window::Window`] and other state.
pub struct Window {
    raw: Box<dyn winit::window::Window>,
    focused: bool,
    theme: Theme,
    /// If the window is completely hidden (minized or covered by another
    /// window)
    occluded: bool,
}

impl Window {
    /// Creates a new default window object using the [`winit::window::Window`].
    #[must_use]
    pub fn new(window: Box<dyn winit::window::Window>) -> Self {
        Self {
            focused: false,
            theme: window.theme().unwrap_or(Theme::Dark),
            occluded: false,
            raw: window,
        }
    }
    /// Returns the unique ID of the window
    #[must_use]
    pub fn id(&self) -> WindowId {
        self.raw.id()
    }
    /// Returns the size of the window's renderable surface. Used by
    /// renderer to create correct surface sizes
    #[must_use]
    pub fn size(&self) -> PhysicalSize<u32> {
        self.raw.surface_size()
    }
    /// Handles [`WindowEvent`]. These should first be proccessed
    /// and accepted by the window.
    pub fn handle_event(&mut self, event: &WindowEvent) {
        if *event.id() != self.id() {
            return;
        }

        match *event {
            // Resizing is handled by the renderer.
            WindowEvent::Resized { .. } => {}
            WindowEvent::Occluded { id: _, occluded } => self.occluded = occluded,
            WindowEvent::FocusChanged { id: _, focused } => self.focused = focused,
            WindowEvent::ThemeChanged { id: _, theme } => self.theme = theme,
        }
    }
}

impl HasWindowHandle for Window {
    fn window_handle(
        &self,
    ) -> Result<winit::raw_window_handle::WindowHandle<'_>, winit::raw_window_handle::HandleError>
    {
        self.raw.window_handle()
    }
}

impl HasDisplayHandle for Window {
    fn display_handle(
        &self,
    ) -> Result<winit::raw_window_handle::DisplayHandle<'_>, winit::raw_window_handle::HandleError>
    {
        self.raw.display_handle()
    }
}
