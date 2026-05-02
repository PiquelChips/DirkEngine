use events::Event;
use winit::{
    event::{ButtonSource, MouseScrollDelta},
    keyboard::{Key, ModifiersState, PhysicalKey},
    window::WindowId,
};

/// An event to signal that the application has exited
#[derive(Debug, Clone, Event)]
#[event("App Exit with code {0}")]
pub struct AppExit(pub i32);

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
    Lines { x: f32, y: f32 },
    /// Scroll expressed in physical pixels (e.g. a trackpad).
    Pixels { x: f64, y: f64 },
}

impl From<MouseScrollDelta> for ScrollDelta {
    fn from(delta: MouseScrollDelta) -> Self {
        match delta {
            MouseScrollDelta::LineDelta(x, y) => Self::Lines { x, y },
            MouseScrollDelta::PixelDelta(px) => Self::Pixels { x: px.x, y: px.y },
        }
    }
}

#[derive(Debug, Clone, Event)]
pub enum InputEvent {
    /// A key was pressed. `repeat` is true for OS-generated key repeat events.
    KeyPressed {
        id: WindowId,
        key: Key,
        physical_key: PhysicalKey,
        modifiers: ModifiersState,
        repeat: bool,
    },
    /// A key was released.
    KeyReleased {
        id: WindowId,
        key: Key,
        physical_key: PhysicalKey,
        modifiers: ModifiersState,
    },
    /// Keyboard modifier keys (Shift, Ctrl, Alt, Super) changed.
    ModifiersChanged {
        id: WindowId,
        modifiers: ModifiersState,
    },
    /// The pointer moved inside the window.
    PointerMoved { id: WindowId, position: glam::DVec2 },
    /// The pointer entered the window area.
    PointerEntered { id: WindowId },
    /// The pointer left the window area.
    PointerLeft { id: WindowId },
    /// A mouse/pointer button was pressed.
    MouseButtonPressed {
        id: WindowId,
        button: ButtonSource,
        position: glam::DVec2,
    },
    /// A mouse/pointer button was released.
    MouseButtonReleased {
        id: WindowId,
        button: ButtonSource,
        position: glam::DVec2,
    },
    /// The scroll wheel or trackpad was scrolled.
    MouseWheelScrolled { id: WindowId, delta: ScrollDelta },
}

impl InputEvent {
    pub fn id(&self) -> &WindowId {
        match self {
            Self::KeyPressed { id, .. } => id,
            Self::KeyReleased { id, .. } => id,
            Self::ModifiersChanged { id, .. } => id,
            Self::PointerMoved { id, .. } => id,
            Self::PointerEntered { id, .. } => id,
            Self::PointerLeft { id, .. } => id,
            Self::MouseButtonPressed { id, .. } => id,
            Self::MouseButtonReleased { id, .. } => id,
            Self::MouseWheelScrolled { id, .. } => id,
        }
    }
}
