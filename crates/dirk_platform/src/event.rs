use dirk_events::Event;
use winit::{
    event::{ButtonSource, MouseScrollDelta},
    keyboard::{Key, ModifiersState, PhysicalKey},
    window::WindowId,
};

/// All platform events.
/// These are specific to global platform stuff. No input or window
/// specific events are listed here.
/// The only exeptions are window closing and creating events.
#[derive(Debug, Clone, Event)]
#[allow(missing_docs)]
pub enum PlatformEvent {
    /// Window created event.
    WindowCreated { id: WindowId },
    /// The OS is asking us to close this window.
    /// This should be when the renderer window is destroyed.
    WindowCloseRequested { id: WindowId },
    /// The window has been finally destroyed. This event should not be
    /// used as all window related objects should have been destroyed on
    /// [`Self::WindowCloseRequested`].
    WindowDestroyed { id: WindowId },
}

/// All window specific events.
#[derive(Debug, Clone, Event)]
#[allow(missing_docs)]
pub enum WindowEvent {
    Resized {
        id: WindowId,
        width: u32,
        height: u32,
    },
    FocusChanged {
        id: WindowId,
        focused: bool,
    },
    Occluded {
        id: WindowId,
        occluded: bool,
    },
    ThemeChanged {
        id: WindowId,
        theme: winit::window::Theme,
    },
}

impl WindowEvent {
    /// Returns the ID of the window referenced in the event. This is
    /// just a match that extracts the ID out of every variant.
    #[must_use]
    pub fn id(&self) -> &WindowId {
        match self {
            Self::Resized { id, .. }
            | Self::Occluded { id, .. }
            | Self::FocusChanged { id, .. }
            | Self::ThemeChanged { id, .. } => id,
        }
    }
}

/// A platform-independent scroll delta.
#[derive(Debug, Clone)]
pub enum ScrollDelta {
    /// Scroll expressed in lines (e.g. a traditional mouse wheel).
    Lines {
        /// Horizontal scroll amount, in lines.
        x: f32,
        /// Vertical scroll amount, in lines.
        y: f32,
    },
    /// Scroll expressed in physical pixels (e.g. a trackpad).
    Pixels {
        /// Horizontal scroll amount, in physical pixels.
        x: f64,
        /// Vertical scroll amount, in physical pixels.
        y: f64,
    },
}

impl From<MouseScrollDelta> for ScrollDelta {
    fn from(delta: MouseScrollDelta) -> Self {
        match delta {
            MouseScrollDelta::LineDelta(x, y) => Self::Lines { x, y },
            MouseScrollDelta::PixelDelta(px) => Self::Pixels { x: px.x, y: px.y },
        }
    }
}

/// Input events routed to UI integrations.
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub enum UiInputEvent {
    WindowFocused {
        id: WindowId,
        focused: bool,
    },
    ModifiersChanged {
        id: WindowId,
        modifiers: ModifiersState,
    },
    Key {
        id: WindowId,
        key: Key,
        physical_key: PhysicalKey,
        pressed: bool,
        repeat: bool,
        modifiers: ModifiersState,
        text: Option<String>,
    },
    PointerMoved {
        id: WindowId,
        position: glam::DVec2,
    },
    PointerGone {
        id: WindowId,
    },
    PointerButton {
        id: WindowId,
        button: ButtonSource,
        position: glam::DVec2,
        pressed: bool,
        modifiers: ModifiersState,
    },
    MouseWheel {
        id: WindowId,
        delta: ScrollDelta,
        modifiers: ModifiersState,
    },
    Ime {
        id: WindowId,
        event: UiImeEvent,
    },
}

impl UiInputEvent {
    /// Returns the window that received this UI input event.
    #[must_use]
    pub fn id(&self) -> WindowId {
        match self {
            Self::WindowFocused { id, .. }
            | Self::ModifiersChanged { id, .. }
            | Self::Key { id, .. }
            | Self::PointerMoved { id, .. }
            | Self::PointerGone { id }
            | Self::PointerButton { id, .. }
            | Self::MouseWheel { id, .. }
            | Self::Ime { id, .. } => *id,
        }
    }
}

/// Input method event routed to UI integrations.
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub enum UiImeEvent {
    Enabled,
    Preedit(String),
    Commit(String),
    Disabled,
}

/// Input events produced by platform windows.
#[derive(Debug, Clone, Event)]
pub enum InputEvent {
    /// A key was pressed. `repeat` is true for OS-generated key repeat events.
    KeyPressed {
        /// Window that received the input.
        id: WindowId,
        /// Logical key after keyboard layout mapping.
        key: Key,
        /// Physical key location, independent of keyboard layout.
        physical_key: PhysicalKey,
        /// Keyboard modifiers active for this input.
        modifiers: ModifiersState,
        /// Whether this is an OS-generated key-repeat press.
        repeat: bool,
    },
    /// A key was released.
    KeyReleased {
        /// Window that received the input.
        id: WindowId,
        /// Logical key after keyboard layout mapping.
        key: Key,
        /// Physical key location, independent of keyboard layout.
        physical_key: PhysicalKey,
        /// Keyboard modifiers active for this input.
        modifiers: ModifiersState,
    },
    /// Keyboard modifier keys (Shift, Ctrl, Alt, Super) changed.
    ModifiersChanged {
        /// Window that received the input.
        id: WindowId,
        /// Current keyboard modifier state.
        modifiers: ModifiersState,
    },
    /// The pointer moved inside the window.
    PointerMoved {
        /// Window that received the input.
        id: WindowId,
        /// Pointer position in physical pixels.
        position: glam::DVec2,
    },
    /// The pointer entered the window area.
    PointerEntered {
        /// Window that received the input.
        id: WindowId,
    },
    /// The pointer left the window area.
    PointerLeft {
        /// Window that received the input.
        id: WindowId,
    },
    /// A mouse/pointer button was pressed.
    MouseButtonPressed {
        /// Window that received the input.
        id: WindowId,
        /// Button that was pressed.
        button: ButtonSource,
        /// Pointer position in physical pixels when the button changed.
        position: glam::DVec2,
    },
    /// A mouse/pointer button was released.
    MouseButtonReleased {
        /// Window that received the input.
        id: WindowId,
        /// Button that was released.
        button: ButtonSource,
        /// Pointer position in physical pixels when the button changed.
        position: glam::DVec2,
    },
    /// The scroll wheel or trackpad was scrolled.
    MouseWheelScrolled {
        /// Window that received the input.
        id: WindowId,
        /// Platform-independent scroll delta.
        delta: ScrollDelta,
    },
}

impl InputEvent {
    /// Returns the window that received this input event.
    #[must_use]
    pub fn id(&self) -> &WindowId {
        match self {
            Self::KeyPressed { id, .. }
            | Self::KeyReleased { id, .. }
            | Self::ModifiersChanged { id, .. }
            | Self::PointerMoved { id, .. }
            | Self::PointerEntered { id }
            | Self::PointerLeft { id }
            | Self::MouseButtonPressed { id, .. }
            | Self::MouseButtonReleased { id, .. }
            | Self::MouseWheelScrolled { id, .. } => id,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{InputCapture, InputRouter};

    use super::*;

    fn window_id(raw: usize) -> WindowId {
        WindowId::from_raw(raw)
    }

    #[test]
    fn router_blocks_pointer_press_when_pointer_capture_is_active() {
        let router = InputRouter::default();
        router.set_capture(
            window_id(1),
            InputCapture {
                pointer: true,
                keyboard: false,
            },
        );

        assert!(!router.should_dispatch_pointer_button(window_id(1), true));
    }

    #[test]
    fn router_allows_pointer_release_when_pointer_capture_is_active() {
        let router = InputRouter::default();
        router.set_capture(
            window_id(1),
            InputCapture {
                pointer: true,
                keyboard: false,
            },
        );

        assert!(router.should_dispatch_pointer_button(window_id(1), false));
    }

    #[test]
    fn router_blocks_key_press_when_keyboard_capture_is_active() {
        let router = InputRouter::default();
        router.set_capture(
            window_id(1),
            InputCapture {
                pointer: false,
                keyboard: true,
            },
        );

        assert!(!router.should_dispatch_key(window_id(1), true));
    }

    #[test]
    fn router_allows_key_release_when_keyboard_capture_is_active() {
        let router = InputRouter::default();
        router.set_capture(
            window_id(1),
            InputCapture {
                pointer: false,
                keyboard: true,
            },
        );

        assert!(router.should_dispatch_key(window_id(1), false));
    }

    #[test]
    fn router_drains_ui_events_in_fifo_order() {
        let router = InputRouter::default();
        router.push_ui_event(UiInputEvent::PointerGone { id: window_id(1) });
        router.push_ui_event(UiInputEvent::PointerGone { id: window_id(2) });

        let events = router.drain_ui_events();

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].id(), window_id(1));
        assert_eq!(events[1].id(), window_id(2));
        assert!(router.drain_ui_events().is_empty());
    }

    #[test]
    fn router_capture_is_scoped_by_window_id() {
        let router = InputRouter::default();
        router.set_capture(
            window_id(1),
            InputCapture {
                pointer: true,
                keyboard: false,
            },
        );

        assert!(router.captures_pointer(window_id(1)));
        assert!(!router.captures_pointer(window_id(2)));
    }
}
