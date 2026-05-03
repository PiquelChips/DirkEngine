//! This is a utility crate.
//! It has basic utilities for use throughout the engine.
//! No actual engine systems live in this crate. It just has
//! many small features, functions and structures.

use std::path::{Path, PathBuf};

mod version;
pub use version::*;

/// The up direction used for all world and
/// renderer coordinate calcualtions.
///
/// Y-up
pub const UP_DIRECTION: glam::Vec3 = glam::Vec3::Y;
/// The forward direction used for all world and
/// renderer coordinate calcualtions.
/// We use Z-forward because that is how Vulkan does it.
pub const FORWARD_DIRECTION: glam::Vec3 = glam::Vec3::Z;

const ROOT: &str = std::env!("WORKSPACE_ROOT");

/// We format the path to make it relative to the workspace root.
///
/// # Errors
///
/// If the path is not relative to the base, an error will be thrown.
pub fn format_path(base: &PathBuf, path: &Path) -> std::io::Result<PathBuf> {
    let root = PathBuf::from(ROOT).join(base);
    Ok(path
        .canonicalize()?
        .strip_prefix(std::env!("WORKSPACE_ROOT"))
        .map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "Path '{}' is not relative to base '{}'",
                    path.display(),
                    root.display()
                ),
            )
        })?
        .to_path_buf())
}
