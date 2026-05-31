//! Per-player input handling.

mod action;
mod axis;

pub use action::{InputAction, InputBinding};
pub use axis::InputAxis;

use std::collections::HashSet;

use dirk_platform::{ButtonSource, InputEvent, KeyCode, MouseButton, PhysicalKey};

/// Per-player input state and bindings.
///
/// Each [`PlayerHandle`] owns one `InputContext`. Movement bindings are still
/// built in, but they are represented as actions and axes so a configurable
/// binding source can replace the defaults later.
///
/// [`PlayerHandle`]: crate::PlayerHandle
#[derive(Default)]
pub struct InputContext {
    snapshot: InputSnapshot,
    movement: MovementBindings,
    last_pointer_position: Option<glam::DVec2>,
    pointer_delta: glam::DVec2,
}

impl InputContext {
    /// Creates an input context with the default player bindings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn handle_event(&mut self, event: &InputEvent) {
        match event {
            InputEvent::KeyPressed { physical_key, .. } => {
                if let PhysicalKey::Code(key) = physical_key {
                    self.snapshot.press(InputBinding::Key(*key));
                }
            }
            InputEvent::KeyReleased { physical_key, .. } => {
                if let PhysicalKey::Code(key) = physical_key {
                    self.snapshot.release(InputBinding::Key(*key));
                }
            }
            InputEvent::MouseButtonPressed {
                button, position, ..
            } => {
                if let ButtonSource::Mouse(button) = button {
                    self.snapshot.press(InputBinding::MouseButton(*button));
                    self.last_pointer_position = Some(*position);
                }
            }
            InputEvent::MouseButtonReleased {
                button, position, ..
            } => {
                if let ButtonSource::Mouse(button) = button {
                    self.snapshot.release(InputBinding::MouseButton(*button));
                    self.last_pointer_position = Some(*position);
                }
            }
            InputEvent::PointerMoved { position, .. } => {
                if let Some(previous) = self.last_pointer_position
                    && self.movement.gate.is_active(&self.snapshot)
                {
                    self.pointer_delta += *position - previous;
                }
                self.last_pointer_position = Some(*position);
            }
            InputEvent::ModifiersChanged { .. } | InputEvent::MouseWheelScrolled { .. } => {}
            InputEvent::PointerEntered { .. } | InputEvent::PointerLeft { .. } => {
                self.last_pointer_position = None;
            }
        }
    }

    #[must_use]
    pub(crate) fn movement_input(&self) -> glam::Vec3 {
        self.movement.value(&self.snapshot)
    }

    #[must_use]
    pub(crate) fn look_input(&self) -> glam::DVec2 {
        self.pointer_delta
    }

    pub(crate) fn clear_frame_state(&mut self) {
        self.pointer_delta = glam::DVec2::ZERO;
    }
}

/// The current state of input.
/// This holds all the keys that are currently being pressed.
#[derive(Debug, Default)]
pub(crate) struct InputSnapshot {
    held: HashSet<InputBinding>,
}

impl InputSnapshot {
    fn press(&mut self, binding: InputBinding) {
        self.held.insert(binding);
    }

    fn release(&mut self, binding: InputBinding) {
        self.held.remove(&binding);
    }

    fn is_held(&self, binding: InputBinding) -> bool {
        self.held.contains(&binding)
    }
}

// TODO: everything below should be part of the dynamic keybinding
// configuration system

#[derive(Debug, Clone)]
struct MovementBindings {
    gate: InputAction,
    right: InputAxis,
    up: InputAxis,
    forward: InputAxis,
}

impl MovementBindings {
    fn value(&self, input: &InputSnapshot) -> glam::Vec3 {
        if !self.gate.is_active(input) {
            return glam::Vec3::ZERO;
        }

        glam::vec3(
            self.right.value(input),
            self.up.value(input),
            self.forward.value(input),
        )
    }
}

impl Default for MovementBindings {
    fn default() -> Self {
        Self {
            gate: InputAction::new("move_gate", [InputBinding::MouseButton(MouseButton::Right)]),
            right: InputAxis::digital(
                "move_right",
                InputAction::new("move_left", [InputBinding::Key(KeyCode::KeyQ)]),
                InputAction::new("move_right", [InputBinding::Key(KeyCode::KeyD)]),
            ),
            up: InputAxis::digital(
                "move_up",
                InputAction::new("move_down", [InputBinding::Key(KeyCode::KeyC)]),
                InputAction::new("move_up", [InputBinding::Key(KeyCode::Space)]),
            ),
            forward: InputAxis::digital(
                "move_forward",
                InputAction::new("move_backward", [InputBinding::Key(KeyCode::KeyS)]),
                InputAction::new("move_forward", [InputBinding::Key(KeyCode::KeyW)]),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn movement_requires_right_mouse_gate() {
        let mut input = InputSnapshot::default();
        input.press(InputBinding::Key(KeyCode::KeyW));

        assert_eq!(MovementBindings::default().value(&input), glam::Vec3::ZERO);
    }

    #[test]
    fn movement_axes_read_default_bindings_when_gate_is_held() {
        let mut input = InputSnapshot::default();
        input.press(InputBinding::MouseButton(MouseButton::Right));
        input.press(InputBinding::Key(KeyCode::KeyW));
        input.press(InputBinding::Key(KeyCode::KeyD));
        input.press(InputBinding::Key(KeyCode::Space));

        assert_eq!(
            MovementBindings::default().value(&input),
            glam::vec3(1.0, 1.0, 1.0)
        );
    }

    #[test]
    fn opposing_axis_bindings_cancel_out() {
        let mut input = InputSnapshot::default();
        input.press(InputBinding::MouseButton(MouseButton::Right));
        input.press(InputBinding::Key(KeyCode::KeyA));
        input.press(InputBinding::Key(KeyCode::KeyD));
        input.press(InputBinding::Key(KeyCode::KeyC));

        assert_eq!(
            MovementBindings::default().value(&input),
            glam::vec3(0.0, -1.0, 0.0)
        );
    }
}
