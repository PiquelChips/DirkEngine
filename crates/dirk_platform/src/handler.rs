use std::collections::HashMap;

use tracing::{debug, trace};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::ModifiersState,
    window::{WindowAttributes, WindowId},
};

use crate::{
    InputEvent, Window,
    event::{PlatformEvent, WindowEvent as PlatformWindowEvent},
};

pub struct PlatformHandler {
    can_create_surfaces: bool,
    pub windows: HashMap<WindowId, Window>,
    main_window: Option<WindowId>,

    /// Current keyboard modifier state, updated on every `ModifiersChanged` event.
    /// TODO: should only be tracked by input manager
    modifiers: ModifiersState,

    /// Dispatch [`PlatformEvent`]
    platform_dispatcher: dirk_events::Dispatcher<PlatformEvent>,
    /// Dispatch [`PlatformWindowEvent`]
    window_dispatcher: dirk_events::Dispatcher<PlatformWindowEvent>,
    /// Dispatch [`InputEvent`]
    input_dispatch: dirk_events::Dispatcher<InputEvent>,
}

impl PlatformHandler {
    pub fn new(events: &dirk_events::EventManager) -> Self {
        Self {
            can_create_surfaces: false,
            windows: HashMap::new(),
            main_window: None,
            modifiers: ModifiersState::default(),
            platform_dispatcher: events.register(),
            window_dispatcher: events.register(),
            input_dispatch: events.register(),
        }
    }
    fn create_window(&mut self, event_loop: &dyn ActiveEventLoop) -> anyhow::Result<WindowId> {
        let window_attributes = WindowAttributes::default()
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

    fn dispatch_platform_event(&mut self, id: WindowId, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::CloseRequested => {
                debug!("Close requested for Window={id:?}");
                self.windows.remove(&id);
                self.platform_dispatcher
                    .dispatch(PlatformEvent::WindowCloseRequested { id });
            }
            WindowEvent::Destroyed => {
                debug!("Window {id:?} destroyed");
                self.platform_dispatcher
                    .dispatch(PlatformEvent::WindowDestroyed { id });
            }
            WindowEvent::SurfaceResized(size) => {
                self.window_dispatcher
                    .dispatch(PlatformWindowEvent::Resized {
                        id,
                        width: size.width,
                        height: size.height,
                    });
            }
            WindowEvent::ThemeChanged(theme) => {
                self.window_dispatcher
                    .dispatch(PlatformWindowEvent::ThemeChanged { id, theme: *theme });
            }
            WindowEvent::Focused(focused) => {
                self.window_dispatcher
                    .dispatch(PlatformWindowEvent::FocusChanged {
                        id,
                        focused: *focused,
                    });
            }
            WindowEvent::Occluded(occluded) => {
                self.window_dispatcher
                    .dispatch(PlatformWindowEvent::Occluded {
                        id,
                        occluded: *occluded,
                    });
            }
            _ => return false,
        }

        true
    }

    fn dispatch_input_event(&mut self, id: WindowId, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::ModifiersChanged(new_modifiers) => {
                self.modifiers = new_modifiers.state();
                trace!("Modifiers changed to {:?}", self.modifiers);
                self.input_dispatch.dispatch(InputEvent::ModifiersChanged {
                    id,
                    modifiers: self.modifiers,
                });
            }
            WindowEvent::KeyboardInput {
                event,
                is_synthetic: false,
                ..
            } => self.dispatch_keyboard_input(id, event),
            WindowEvent::PointerMoved { position, .. } => {
                trace!("Pointer moved to {position:?}");
                self.input_dispatch.dispatch(InputEvent::PointerMoved {
                    id,
                    position: glam::dvec2(position.x, position.y),
                });
            }
            WindowEvent::PointerEntered { .. } => {
                trace!("Pointer entered Window={id:?}");
                self.input_dispatch
                    .dispatch(InputEvent::PointerEntered { id });
            }
            WindowEvent::PointerLeft { .. } => {
                trace!("Pointer left Window={id:?}");
                self.input_dispatch.dispatch(InputEvent::PointerLeft { id });
            }
            WindowEvent::PointerButton {
                button,
                state,
                position,
                ..
            } => {
                trace!("Pointer button {button:?} {state:?} at {position:?}");
                let position = glam::dvec2(position.x, position.y);
                let event = match state {
                    ElementState::Pressed => InputEvent::MouseButtonPressed {
                        id,
                        button: button.clone(),
                        position,
                    },
                    ElementState::Released => InputEvent::MouseButtonReleased {
                        id,
                        button: button.clone(),
                        position,
                    },
                };
                self.input_dispatch.dispatch(event);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                trace!("Mouse wheel {delta:?}");
                self.input_dispatch
                    .dispatch(InputEvent::MouseWheelScrolled {
                        id,
                        delta: (*delta).into(),
                    });
            }
            _ => return false,
        }

        true
    }

    fn dispatch_keyboard_input(&self, id: WindowId, event: &winit::event::KeyEvent) {
        let modifiers = self.modifiers;
        match event.state {
            ElementState::Pressed => {
                trace!(
                    "Key pressed: {:?} (repeat={})",
                    event.logical_key, event.repeat
                );
                self.input_dispatch.dispatch(InputEvent::KeyPressed {
                    id,
                    key: event.logical_key.clone(),
                    physical_key: event.physical_key,
                    modifiers,
                    repeat: event.repeat,
                });
            }
            ElementState::Released => {
                trace!("Key released: {:?}", event.logical_key);
                self.input_dispatch.dispatch(InputEvent::KeyReleased {
                    id,
                    key: event.logical_key.clone(),
                    physical_key: event.physical_key,
                    modifiers,
                });
            }
        }
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
        if self.dispatch_platform_event(id, &event) {
            return;
        }
        self.dispatch_input_event(id, &event);
    }
}
