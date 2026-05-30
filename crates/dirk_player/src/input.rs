//! Per-player input handling.
//!
//! # Structure
//!
//! | Type | Role |
//! |------|------|
//! | [`InputContext`] | Owned by each player; holds bindings and current-frame state |
//! | [`InputAction`] | A named digital input (button pressed / released / held) |
//! | [`InputAxis`] | A named analog input (stick, trigger, mouse delta) |
//!
//! # Status
//!
//! These types are stubs. The public API is intentionally minimal so that
//! call sites compile and have a clear place to grow into.

mod action;
mod axis;

pub use action::InputAction;
pub use axis::InputAxis;

/// Per-player input state and bindings.
///
/// Each [`PlayerHandle`] owns one `InputContext`. When input is implemented,
/// this will map raw platform input events to [`InputAction`]s and
/// [`InputAxis`] values that game systems can read.
///
/// [`PlayerHandle`]: crate::PlayerHandle
#[derive(Default)]
pub struct InputContext {
    // TODO: input bindings (action name → key/button mapping)
    // TODO: current-frame action states
    // TODO: current-frame axis values
}

impl InputContext {
    /// Creates an empty input context with no bindings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}
