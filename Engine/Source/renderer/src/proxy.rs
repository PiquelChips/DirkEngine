//! This module holds proxies for various engine objects

pub mod systems;

mod types;
// TODO: shouldn't be public
pub use types::*;

/// This is the renderer proxy for the [`Universe`]. It also has
/// most of the rendering state needed to render each scene.
pub struct SceneManager {}
