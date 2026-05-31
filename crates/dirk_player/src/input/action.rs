//! Digital input actions.

use dirk_platform::{KeyCode, MouseButton};

use super::InputSnapshot;

/// A raw input that can activate an [`InputAction`].
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum InputBinding {
    /// A physical keyboard key.
    Key(KeyCode),
    /// A mouse button.
    MouseButton(MouseButton),
}

/// A named digital input that is active while any of its bindings are held.
#[derive(Debug, Clone)]
pub struct InputAction {
    name: &'static str,
    bindings: Vec<InputBinding>,
}

impl InputAction {
    /// Creates an action with the supplied bindings.
    #[must_use]
    pub fn new(name: &'static str, bindings: impl Into<Vec<InputBinding>>) -> Self {
        Self {
            name,
            bindings: bindings.into(),
        }
    }

    /// Returns this action's stable name.
    #[must_use]
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Returns the bindings that can activate this action.
    #[must_use]
    pub fn bindings(&self) -> &[InputBinding] {
        &self.bindings
    }

    pub(crate) fn is_active(&self, input: &InputSnapshot) -> bool {
        self.bindings.iter().any(|binding| input.is_held(*binding))
    }
}
