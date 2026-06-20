use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use parking_lot::Mutex;
use winit::window::WindowId;

use crate::UiInputEvent;

/// Whether a UI layer is currently capturing input for a window.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub struct InputCapture {
    /// Pointer input should not be routed to gameplay.
    pub pointer: bool,
    /// Keyboard input should not be routed to gameplay.
    pub keyboard: bool,
}

#[derive(Default)]
struct InputRouterState {
    ui_events: VecDeque<UiInputEvent>,
    captures: HashMap<WindowId, InputCapture>,
}

/// Shared router between platform input and UI integrations.
#[derive(Clone, Default)]
pub struct InputRouter {
    state: Arc<Mutex<InputRouterState>>,
}

impl InputRouter {
    /// Drains all UI input events currently pending in FIFO order.
    #[must_use]
    pub fn drain_ui_events(&self) -> Vec<UiInputEvent> {
        self.state.lock().ui_events.drain(..).collect()
    }

    /// Sets the current input capture state for one window.
    pub fn set_capture(&self, id: WindowId, capture: InputCapture) {
        self.state.lock().captures.insert(id, capture);
    }

    pub(crate) fn push_ui_event(&self, event: UiInputEvent) {
        self.state.lock().ui_events.push_back(event);
    }

    pub(crate) fn captures_pointer(&self, id: WindowId) -> bool {
        self.state
            .lock()
            .captures
            .get(&id)
            .is_some_and(|capture| capture.pointer)
    }

    pub(crate) fn captures_keyboard(&self, id: WindowId) -> bool {
        self.state
            .lock()
            .captures
            .get(&id)
            .is_some_and(|capture| capture.keyboard)
    }

    pub(crate) fn should_dispatch_pointer_button(&self, id: WindowId, pressed: bool) -> bool {
        !pressed || !self.captures_pointer(id)
    }

    pub(crate) fn should_dispatch_key(&self, id: WindowId, pressed: bool) -> bool {
        !pressed || !self.captures_keyboard(id)
    }
}
