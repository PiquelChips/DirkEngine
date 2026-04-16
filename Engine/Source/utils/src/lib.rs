//! This is a utility crate.
//! It has basic utilities for use throughout the engine.
//! No actual engine systems live in this crate. It just has
//! many small features, functions and structures.

/// The up direction used for all world and
/// renderer coordinate calcualtions.
///
/// Y-up
pub const UP_DIRECTION: glam::Vec3 = glam::Vec3::Y;
/// The forward direction used for all world and
/// renderer coordinate calcualtions.
/// We use Z-forward because that is how Vulkan does it.
pub const FORWARD_DIRECTION: glam::Vec3 = glam::Vec3::Z;
