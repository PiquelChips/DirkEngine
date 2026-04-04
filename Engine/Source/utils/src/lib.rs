//! This is a utility crate.
//! It has basic utilities for use throughout the engine.
//! No actual engine systems live in this crate. It just has
//! many small features, functions and structures.

mod version;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
pub use version::*;

pub trait Window: HasWindowHandle + HasDisplayHandle {
    fn needs_resize(&self) -> bool;
    /// Returns (width, height)
    fn get_window_size(&self) -> (u32, u32);
}
