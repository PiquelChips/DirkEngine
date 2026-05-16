use std::collections::HashMap;

use tracing::{debug, trace};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{WindowAttributes, WindowId},
};

use crate::{
    Window,
    event::{PlatformEvent, WindowEvent as PlatformWindowEvent},
};

pub struct PlatformHandler {
    can_create_surfaces: bool,
    pub windows: HashMap<WindowId, Window>,
    main_window: Option<WindowId>,

    /// Dispatch [`PlatformEvent`]
    platform_dispatcher: dirk_events::Dispatcher<PlatformEvent>,
    /// Dispatch [`PlatformWindowEvent`]
    window_dispatcher: dirk_events::Dispatcher<PlatformWindowEvent>,
}

impl PlatformHandler {
    pub fn new(events: &dirk_events::EventManager) -> Self {
        Self {
            can_create_surfaces: false,
            windows: HashMap::new(),
            main_window: None,
            platform_dispatcher: events.register(),
            window_dispatcher: events.register(),
        }
    }
    fn create_window(&mut self, event_loop: &dyn ActiveEventLoop) -> anyhow::Result<WindowId> {
        #[allow(unused_mut)]
        let mut window_attributes = WindowAttributes::default()
            .with_title("DirkEngine")
            .with_transparent(true);

        let window = event_loop.create_window(window_attributes)?;

        let window = Window::new(window);
        let window_id = window.id();
        self.platform_dispatcher
            .dispatch(PlatformEvent::WindowCreated { id: window_id });
        self.windows.insert(window_id, window);
        Ok(window_id)
    }
    pub fn main_window(&self) -> &Window {
        self.windows
            .get(
                &self
                    .main_window
                    .expect("there should always be a main window"),
            )
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

impl ApplicationHandler for PlatformHandler {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        let id = self
            .create_window(event_loop)
            .expect("failed to create main window");
        self.main_window = Some(id);
        self.can_create_surfaces = true;
    }

    fn window_event(&mut self, _loop: &dyn ActiveEventLoop, id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                debug!("Close requested for Window={id:?}");
                self.windows.remove(&id);
                self.platform_dispatcher.dispatch(PlatformEvent::WindowCloseRequested { id });
            }
            WindowEvent::Destroyed => {
                debug!("Window {id:?} destroyed");
                self.platform_dispatcher .dispatch(PlatformEvent::WindowDestroyed { id });
            }
            WindowEvent::SurfaceResized(size) => {
                self.window_dispatcher.dispatch(PlatformWindowEvent::Resized {
                    id,
                    width: size.width,
                    height: size.height,
                });
            }
            WindowEvent::ThemeChanged(theme) => {
                self.window_dispatcher.dispatch(PlatformWindowEvent::ThemeChanged { id, theme });
            }
            WindowEvent::Focused(focused) => {
                self.window_dispatcher.dispatch(PlatformWindowEvent::FocusChanged { id, focused });
            }
            WindowEvent::Occluded(occluded) => {
                self.window_dispatcher.dispatch(PlatformWindowEvent::Occluded { id, occluded });
                }
            WindowEvent::ModifiersChanged(_modifiers) => {
            /*
                window.set_modifiers(modifiers.state());
                trace!("Modifiers changed to {:?}", window.get_modifiers());
            */
            }
            WindowEvent::MouseWheel { delta, .. } => {
                trace!("Mouse wheel event: {delta:?}");
            }
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
                event,
                is_synthetic,
                ..
            } => {
                trace!("Input Event: {:?} {:?}, {is_synthetic}", event.logical_key, event.state);
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
                trace!("Pointer left Window={id:?}");
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
            | WindowEvent::PointerEntered { .. }
            | WindowEvent::DragEntered { .. }
            | WindowEvent::DragMoved { .. }
            | WindowEvent::DragDropped { .. }
            | WindowEvent::ScaleFactorChanged { .. }
            // Drawing is handled by the main engine loop. Redraw requests
            // are thus ignored.
            | WindowEvent::RedrawRequested
            | WindowEvent::Ime(_)
            | WindowEvent::Moved(_) => {},
        }
    }
}
