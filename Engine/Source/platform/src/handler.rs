use std::collections::HashMap;

use log::{debug, trace};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{WindowAttributes, WindowId},
};

use crate::window::Window;

/// The object with actual platform state. It handles responding to platform events.
pub struct PlatformHandler {
    can_create_surfaces: bool,
    windows: HashMap<WindowId, Window>,
    main_window: WindowId,
}

impl PlatformHandler {
    fn create_window(&mut self, event_loop: &dyn ActiveEventLoop) -> anyhow::Result<WindowId> {
        #[allow(unused_mut)]
        let mut window_attributes = WindowAttributes::default()
            .with_title("DirkEngine")
            .with_transparent(true);

        let window = event_loop.create_window(window_attributes)?;

        let window = Window::new(window);
        let window_id = window.id();
        debug!("Created new window with id={window_id:?}");
        self.windows.insert(window_id, window);
        Ok(window_id)
    }
    pub fn main_window(&self) -> &Window {
        self.windows
            .get(&self.main_window)
            .expect("there should always be a main window")
    }
    pub fn is_initialized(&self) -> bool {
        self.can_create_surfaces
    }
    pub fn shutdown(&mut self) {
        let count = self.windows.len();
        self.windows.clear();
        debug!("Closed {count} window(s) during platform shutdown");
    }
}

impl Default for PlatformHandler {
    fn default() -> Self {
        Self {
            can_create_surfaces: false,
            windows: HashMap::new(),
            main_window: WindowId::from_raw(0),
        }
    }
}

impl ApplicationHandler for PlatformHandler {
    fn can_create_surfaces(&mut self, event_loop: &dyn winit::event_loop::ActiveEventLoop) {
        self.main_window = self
            .create_window(event_loop)
            .expect("failed to create main window");
        self.can_create_surfaces = true
    }
    fn window_event(
        &mut self,
        _event_loop: &dyn winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let window = match self.windows.get_mut(&window_id) {
            Some(window) => window,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested | WindowEvent::Destroyed => {
                debug!("Closing Window={window_id:?}");
                self.windows.remove(&window_id);
                todo!("if main window: shut down the engine")
            }
            WindowEvent::SurfaceResized(size) => {
                window.resize(size);
            }
            WindowEvent::Focused(focused) => {
                if focused {
                    trace!("Window={window_id:?} focused");
                } else {
                    trace!("Window={window_id:?} unfocused");
                }
                window.focused(focused);
            }
            WindowEvent::ThemeChanged(theme) => {
                trace!("Theme changed to {theme:?}");
                window.set_draw_theme(theme);
            }
            WindowEvent::Occluded(occluded) => {
                window.set_occluded(occluded);
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                window.set_modifiers(modifiers.state());
                trace!("Modifiers changed to {:?}", window.get_modifiers());
            }
            WindowEvent::MouseWheel { delta: _, .. } => {}
            /* TODO: input events
            match delta {
                MouseScrollDelta::LineDelta(x, y) => {
                    trace!("Mouse wheel Line Delta: ({x},{y})");
                }
                MouseScrollDelta::PixelDelta(px) => {
                    trace!("Mouse wheel Pixel Delta: ({},{})", px.x, px.y);
                }
            },
            */
            WindowEvent::KeyboardInput {
                event: _,
                is_synthetic: false,
                ..
            } => {
                /* TODO: input events
                let mods = window.modifiers;

                // Dispatch actions only on press.
                if event.state.is_pressed() {
                    let action = if let Key::Character(ch) = event.key_without_modifiers.as_ref() {
                        Self::process_key_binding(&ch.to_uppercase(), &mods)
                    } else {
                        None
                    };

                    if let Some(action) = action {
                        self.handle_action_with_window(event_loop, window_id, action);
                    }
                }
                */
            }
            WindowEvent::PointerButton { button, state, .. } => {
                trace!("Pointer button {button:?} {state:?}");
                /* TODO: input events
                let mods = window.modifiers;
                if let Some(action) = state
                    .is_pressed()
                    .then(|| button.mouse_button())
                    .flatten()
                    .and_then(|button| Self::process_mouse_binding(button, &mods))
                {
                    self.handle_action_with_window(event_loop, window_id, action);
                }
                */
            }
            WindowEvent::PointerLeft { .. } => {
                trace!("Pointer left Window={window_id:?}");
                // TODO: input events: window.cursor_left();
            }
            WindowEvent::PointerMoved { position, .. } => {
                trace!("Moved pointer to {position:?}");
                // TODO: input events: window.cursor_moved(position);
            }
            WindowEvent::ActivationTokenDone { token: _token, .. } => {
                /* TODO: activation token (X11/Wayland)
                #[cfg(any(x11_platform, wayland_platform))]
                {
                    startup_notify::set_activation_token_env(_token);
                    if let Err(err) = self.create_window(event_loop, None) {
                        error!("Error creating new window: {err}");
                    }
                }
                */
            }
            WindowEvent::PinchGesture { .. }
            | WindowEvent::RotationGesture { .. }
            | WindowEvent::PanGesture { .. }
            | WindowEvent::DoubleTapGesture { .. }
            | WindowEvent::TouchpadPressure { .. }
            | WindowEvent::DragLeft { .. }
            | WindowEvent::KeyboardInput { .. }
            | WindowEvent::PointerEntered { .. }
            | WindowEvent::DragEntered { .. }
            | WindowEvent::DragMoved { .. }
            | WindowEvent::DragDropped { .. }
            | WindowEvent::ScaleFactorChanged { .. }
            // Drawing is handled by the main engine loop. Redraw requests
            // are thus ignored.
            | WindowEvent::RedrawRequested
            | WindowEvent::Ime(_)
            | WindowEvent::Moved(_) => (),
        }
    }
}
