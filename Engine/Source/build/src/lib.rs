//! This crate has a bunch of utilities for the engine build scripts.
//! This mainly avoids duplicating the code of stuff like platform
//! configuration and the editor.

/// Setup build for platform configuration.
/// Run this function in build.rs of a crate that
/// runs platform specific code.
pub fn configure_platform() {
    // declare the config keys for platform specific conditional compilation
    println!("cargo:rustc-check-cfg=cfg(platform_windows)");
    println!("cargo:rustc-check-cfg=cfg(platform_linux)");
    println!("cargo:rustc-check-cfg=cfg(platform_macos)");
    println!("cargo:rustc-check-cfg=cfg(platform_android)");
    println!("cargo:rustc-check-cfg=cfg(platform_ios)");
}

/// Setup build for editor configuration.
/// Run this function in build.rs of a create that
/// runs editor (or not) specific code.
pub fn configure_editor() {
    // declare editor config keys
    println!("cargo:rustc-check-cfg=cfg(editor)");

    // always build as editor for now
    println!("cargo:rustc-cfg=editor");
}

/// Gets the path where all runtime generated files are
/// stored (cache, logs, ...).
#[must_use]
pub fn get_run_dir() -> String {
    String::from("Saved")
}
