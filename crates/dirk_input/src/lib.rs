#![doc = include_str!("../README.md")]

#[cfg(feature = "egui")]
pub mod egui;

/// Whether a button-like input is pressed or released.
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum ButtonState {
    /// The input is pressed.
    Pressed,
    /// The input is released.
    Released,
}

/// Keyboard input after layout mapping.
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum LogicalKey {
    /// A text-producing key. Stored lower-case for binding comparison.
    Character(String),
    /// A named non-text key.
    Named(NamedKey),
}

impl LogicalKey {
    /// Creates a normalized character key.
    #[must_use]
    pub fn character(text: impl AsRef<str>) -> Self {
        Self::Character(text.as_ref().to_lowercase())
    }
}

/// Named logical keyboard keys used by engine bindings.
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum NamedKey {
    /// Escape.
    Escape,
    /// Tab.
    Tab,
    /// Enter/Return.
    Enter,
    /// Backspace.
    Backspace,
    /// Space.
    Space,
    /// Up arrow.
    ArrowUp,
    /// Down arrow.
    ArrowDown,
    /// Left arrow.
    ArrowLeft,
    /// Right arrow.
    ArrowRight,
    /// Home.
    Home,
    /// End.
    End,
    /// Page up.
    PageUp,
    /// Page down.
    PageDown,
    /// Insert.
    Insert,
    /// Delete.
    Delete,
    /// Function key, numbered from 1.
    Function(u8),
}

/// Pointer or mouse button.
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum PointerButton {
    /// Primary pointer button, usually left mouse.
    Primary,
    /// Secondary pointer button, usually right mouse.
    Secondary,
    /// Middle pointer button.
    Middle,
    /// Back/extra button.
    Back,
    /// Forward/extra button.
    Forward,
    /// Any other button code.
    Other(u16),
}

/// Keyboard modifiers active during an input event.
#[derive(Debug, Default, Clone, Copy, Eq, Hash, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
pub struct Modifiers {
    /// Alt/Option.
    pub alt: bool,
    /// Control.
    pub ctrl: bool,
    /// Shift.
    pub shift: bool,
    /// Super/Command/Windows.
    pub super_key: bool,
}

/// Viewport-local normalized pointer position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NormalizedPosition(pub glam::Vec2);

impl NormalizedPosition {
    /// Creates a clamped normalized position.
    #[must_use]
    pub fn new(position: glam::Vec2) -> Self {
        Self(position.clamp(glam::Vec2::ZERO, glam::Vec2::ONE))
    }
}

/// Viewport-local normalized delta.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NormalizedDelta(pub glam::Vec2);

/// Engine input event. Contains no platform window identity.
#[derive(Debug, Clone, PartialEq)]
pub enum InputEvent {
    /// A logical key changed state.
    Key {
        /// Logical key after layout mapping.
        key: LogicalKey,
        /// Button state.
        state: ButtonState,
        /// Whether this is an OS-generated repeat.
        repeat: bool,
        /// Active keyboard modifiers.
        modifiers: Modifiers,
    },
    /// The pointer moved.
    PointerMoved {
        /// Current clamped normalized position.
        position: NormalizedPosition,
        /// Normalized delta since the previous pointer position.
        delta: NormalizedDelta,
    },
    /// The pointer entered the input region.
    PointerEntered,
    /// The pointer left the input region.
    PointerLeft,
    /// A pointer button changed state.
    PointerButton {
        /// Pointer button.
        button: PointerButton,
        /// Button state.
        state: ButtonState,
        /// Current clamped normalized position.
        position: NormalizedPosition,
    },
    /// Scroll wheel or trackpad input, normalized by viewport/window size.
    Scroll {
        /// Normalized scroll delta.
        delta: NormalizedDelta,
    },
}
