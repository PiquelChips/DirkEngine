use log::debug;
use winit::{
    dpi::PhysicalSize,
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window::{Theme, WindowId},
};

use crate::event::WindowEvent;

pub struct Window {
    window: Box<dyn winit::window::Window>,
    focused: bool,
    theme: Theme,
    /// If the window is completely hidden (minized or covered by another
    /// window)
    occluded: bool,

    window_consumer: events::Consumer<WindowEvent>,
}

impl Window {
    pub fn new(window: Box<dyn winit::window::Window>) -> Self {
        Self {
            focused: false,
            theme: window.theme().unwrap_or(Theme::Dark),
            occluded: false,
            window,
            // TODO: window_consumer
        }
    }
    pub fn id(&self) -> WindowId {
        self.window.id()
    }
    pub fn size(&self) -> PhysicalSize<u32> {
        self.window.surface_size()
    }

    pub fn tick(&mut self, _delta_time: f32) {
        let events: Vec<WindowEvent> = self.window_consumer.consume_all().collect();

        for event in events {
            self.handle_event(event);
        }
    }
    /// Handles [WindowEvent]. These should first be proccessed
    /// and accepted by the window.
    fn handle_event(&mut self, event: WindowEvent) {
        if *event.id() != self.id() {
            return;
        }

        match event {
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
