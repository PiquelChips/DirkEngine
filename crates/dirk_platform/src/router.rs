use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use parking_lot::Mutex;
use winit::window::WindowId;

use crate::{InputEvent, UiInputEvent};

/// Whether a UI layer is currently capturing input for a window.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub struct InputCapture {
    /// Pointer input should not be routed to gameplay.
    pub pointer: bool,
    /// Keyboard input should not be routed to gameplay.
    pub keyboard: bool,
}

struct InputRouterState {
    ui_events: VecDeque<UiInputEvent>,
    input_events: VecDeque<InputEvent>,
    captures: HashMap<WindowId, InputCapture>,
    direct_input_dispatch: bool,
}

impl Default for InputRouterState {
    fn default() -> Self {
        Self {
            ui_events: VecDeque::default(),
            input_events: VecDeque::default(),
            captures: HashMap::default(),
            direct_input_dispatch: true,
        }
    }
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

    /// Drains all platform input events currently pending in FIFO order.
    #[must_use]
    pub fn drain_input_events(&self) -> Vec<InputEvent> {
        self.state.lock().input_events.drain(..).collect()
    }

    /// Sets the current input capture state for one window.
    pub fn set_capture(&self, id: WindowId, capture: InputCapture) {
        self.state.lock().captures.insert(id, capture);
    }

    /// Enables or disables direct dispatch of platform input events.
    ///
    /// UI/editor integrations can disable direct dispatch and route the raw
    /// input events themselves after they know which view or widget owns input.
    pub fn set_direct_input_dispatch(&self, enabled: bool) {
        self.state.lock().direct_input_dispatch = enabled;
    }

    pub(crate) fn push_ui_event(&self, event: UiInputEvent) {
        self.state.lock().ui_events.push_back(event);
    }

    pub(crate) fn push_input_event(&self, event: InputEvent) {
        self.state.lock().input_events.push_back(event);
    }

    pub(crate) fn direct_input_dispatch_enabled(&self) -> bool {
        self.state.lock().direct_input_dispatch
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
