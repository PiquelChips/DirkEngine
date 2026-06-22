use tracing::{debug, trace};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::ModifiersState,
    window::{WindowAttributes, WindowId},
};

use crate::{
    InputEvent, InputRouter, PlatformWindows, UiImeEvent, UiInputEvent, Window,
    event::{PlatformEvent, WindowEvent as PlatformWindowEvent},
};

pub struct PlatformHandler {
    can_create_surfaces: bool,
    windows: PlatformWindows,

    /// Current keyboard modifier state, updated on every `ModifiersChanged` event.
    /// TODO: should only be tracked by input manager
    modifiers: ModifiersState,

    /// Dispatch [`PlatformEvent`]
    platform_dispatcher: dirk_events::Dispatcher<PlatformEvent>,
    /// Dispatch [`PlatformWindowEvent`]
    window_dispatcher: dirk_events::Dispatcher<PlatformWindowEvent>,
    /// Dispatch [`InputEvent`]
    input_dispatch: dirk_events::Dispatcher<InputEvent>,
    input_router: InputRouter,
}

impl PlatformHandler {
    pub fn new(
        events: &dirk_events::EventManager,
        windows: PlatformWindows,
        input_router: InputRouter,
    ) -> Self {
        Self {
            can_create_surfaces: false,
            windows,
            modifiers: ModifiersState::default(),
            platform_dispatcher: events.register(),
            window_dispatcher: events.register(),
            input_dispatch: events.register(),
            input_router,
        }
    }
    fn create_window(&mut self, event_loop: &dyn ActiveEventLoop) -> anyhow::Result<WindowId> {
        let window_attributes = WindowAttributes::default()
            .with_title("DirkEngine")
            .with_transparent(true);

        let window = event_loop.create_window(window_attributes)?;

        let window = Window::new(window);
        let window_id = window.id();
        self.windows.insert(window);
        self.platform_dispatcher
            .dispatch(PlatformEvent::WindowCreated { id: window_id });
        Ok(window_id)
    }
    pub fn is_initialized(&self) -> bool {
        self.can_create_surfaces
    }
    pub fn shutdown(&mut self) {
        let count = self.windows.clear();
        debug!("Closed {count} window(s) during platform shutdown");
    }

    fn dispatch_platform_event(&mut self, id: WindowId, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::CloseRequested => {
                debug!("Close requested for Window={id:?}");
                self.windows.remove(id);
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
                self.input_router
                    .push_ui_event(UiInputEvent::WindowFocused {
                        id,
                        focused: *focused,
                    });
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
                self.input_router
                    .push_ui_event(UiInputEvent::ModifiersChanged {
                        id,
                        modifiers: self.modifiers,
                    });
                let event = InputEvent::ModifiersChanged {
                    id,
                    modifiers: self.modifiers,
                };
                self.input_router.push_input_event(event.clone());
                if self.input_router.direct_input_dispatch_enabled() {
                    self.input_dispatch.dispatch(event);
                }
            }
            WindowEvent::KeyboardInput {
                event,
                is_synthetic: false,
                ..
            } => self.dispatch_keyboard_input(id, event),
            WindowEvent::PointerMoved { position, .. } => self.dispatch_pointer_moved(id, position),
            WindowEvent::PointerEntered { position, .. } => {
                self.dispatch_pointer_entered(id, position);
            }
            WindowEvent::PointerLeft { .. } => self.dispatch_pointer_left(id),
            WindowEvent::PointerButton {
                button,
                state,
                position,
                ..
            } => self.dispatch_pointer_button(id, button, *state, position),
            WindowEvent::MouseWheel { delta, .. } => self.dispatch_mouse_wheel(id, delta),
            WindowEvent::Ime(event) => self.dispatch_ime(id, event),
            _ => return false,
        }

        true
    }

    fn dispatch_pointer_moved(&self, id: WindowId, position: &winit::dpi::PhysicalPosition<f64>) {
        trace!("Pointer moved to {position:?}");
        let position = glam::dvec2(position.x, position.y);
        self.input_router
            .push_ui_event(UiInputEvent::PointerMoved { id, position });
        let event = InputEvent::PointerMoved { id, position };
        self.input_router.push_input_event(event.clone());
        if self.input_router.direct_input_dispatch_enabled()
            && !self.input_router.captures_pointer(id)
        {
            self.input_dispatch.dispatch(event);
        }
    }

    fn dispatch_pointer_entered(&self, id: WindowId, position: &winit::dpi::PhysicalPosition<f64>) {
        trace!("Pointer entered Window={id:?}");
        self.input_router.push_ui_event(UiInputEvent::PointerMoved {
            id,
            position: glam::dvec2(position.x, position.y),
        });
        let event = InputEvent::PointerEntered { id };
        self.input_router.push_input_event(event.clone());
        if self.input_router.direct_input_dispatch_enabled() {
            self.input_dispatch.dispatch(event);
        }
    }

    fn dispatch_pointer_left(&self, id: WindowId) {
        trace!("Pointer left Window={id:?}");
        self.input_router
            .push_ui_event(UiInputEvent::PointerGone { id });
        let event = InputEvent::PointerLeft { id };
        self.input_router.push_input_event(event.clone());
        if self.input_router.direct_input_dispatch_enabled() {
            self.input_dispatch.dispatch(event);
        }
    }

    fn dispatch_pointer_button(
        &self,
        id: WindowId,
        button: &winit::event::ButtonSource,
        state: ElementState,
        position: &winit::dpi::PhysicalPosition<f64>,
    ) {
        trace!("Pointer button {button:?} {state:?} at {position:?}");
        let position = glam::dvec2(position.x, position.y);
        let pressed = state == ElementState::Pressed;
        self.input_router
            .push_ui_event(UiInputEvent::PointerButton {
                id,
                button: button.clone(),
                position,
                pressed,
                modifiers: self.modifiers,
            });
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
        self.input_router.push_input_event(event.clone());
        if self.input_router.direct_input_dispatch_enabled()
            && self
                .input_router
                .should_dispatch_pointer_button(id, pressed)
        {
            self.input_dispatch.dispatch(event);
        }
    }

    fn dispatch_mouse_wheel(&self, id: WindowId, delta: &winit::event::MouseScrollDelta) {
        trace!("Mouse wheel {delta:?}");
        let delta: crate::ScrollDelta = (*delta).into();
        self.input_router.push_ui_event(UiInputEvent::MouseWheel {
            id,
            delta: delta.clone(),
            modifiers: self.modifiers,
        });
        let event = InputEvent::MouseWheelScrolled { id, delta };
        self.input_router.push_input_event(event.clone());
        if self.input_router.direct_input_dispatch_enabled()
            && !self.input_router.captures_pointer(id)
        {
            self.input_dispatch.dispatch(event);
        }
    }

    fn dispatch_ime(&self, id: WindowId, event: &winit::event::Ime) {
        let event = match event {
            winit::event::Ime::Enabled => UiImeEvent::Enabled,
            winit::event::Ime::Preedit(text, _) => UiImeEvent::Preedit(text.clone()),
            winit::event::Ime::Commit(text) => UiImeEvent::Commit(text.clone()),
            winit::event::Ime::Disabled => UiImeEvent::Disabled,
            winit::event::Ime::DeleteSurrounding { .. } => return,
        };
        self.input_router
            .push_ui_event(UiInputEvent::Ime { id, event });
    }

    fn dispatch_keyboard_input(&self, id: WindowId, event: &winit::event::KeyEvent) {
        let modifiers = self.modifiers;
        let pressed = event.state == ElementState::Pressed;
        self.input_router.push_ui_event(UiInputEvent::Key {
            id,
            key: event.logical_key.clone(),
            physical_key: event.physical_key,
            pressed,
            repeat: event.repeat,
            modifiers,
            text: event.text.as_ref().map(ToString::to_string),
        });
        match event.state {
            ElementState::Pressed => {
                trace!(
                    "Key pressed: {:?} (repeat={})",
                    event.logical_key, event.repeat
                );
                let event = InputEvent::KeyPressed {
                    id,
                    key: event.logical_key.clone(),
                    physical_key: event.physical_key,
                    modifiers,
                    repeat: event.repeat,
                };
                self.input_router.push_input_event(event.clone());
                if self.input_router.direct_input_dispatch_enabled()
                    && self.input_router.should_dispatch_key(id, pressed)
                {
                    self.input_dispatch.dispatch(event);
                }
            }
            ElementState::Released => {
                trace!("Key released: {:?}", event.logical_key);
                let event = InputEvent::KeyReleased {
                    id,
                    key: event.logical_key.clone(),
                    physical_key: event.physical_key,
                    modifiers,
                };
                self.input_router.push_input_event(event.clone());
                if self.input_router.direct_input_dispatch_enabled() {
                    self.input_dispatch.dispatch(event);
                }
            }
        }
    }
}

impl ApplicationHandler for PlatformHandler {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        let id = self
            .create_window(event_loop)
            .expect("failed to create main window");
        self.windows.set_main_window(id);
        self.can_create_surfaces = true;
    }

    fn window_event(&mut self, _loop: &dyn ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if self.dispatch_platform_event(id, &event) {
            return;
        }
        self.dispatch_input_event(id, &event);
    }
}
