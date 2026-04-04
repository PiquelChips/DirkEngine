//! This crate has the ResourceManager struct.
//! This struct handles all loading of data from the
//! file system. It is intimately linked with the platform crate
//! for platform-specifc resource loading.
//!
//! This crate can load glTF models into internal structs for upload by
//! the renderer.
//! It can also load textures for use by the renderer.
//!
//! In the future, it will also load sound and other assets. However,
//! as these systems aren't implemented yet, the resource manager does
//! not support loading them.

mod errors;
pub use errors::Error;
use errors::Result;

const ASSETS_PATH: &str = env!("ASSETS_PATH");
const MODELS_PATH: &str = env!("MODELS_PATH");
/// This is the main struct that handles loading resources.
pub struct ResourceManager {}
