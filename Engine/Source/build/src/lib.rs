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

/// Adds the paths for assets & models.
///
/// # Panics
///
/// Will panic if the `CARGO_MANIFEST_DIR` env var is not set
pub fn setup_assets() {
    let assets_path = format!(
        "{}/../../Assets",
        std::env::var("CARGO_MANIFEST_DIR").expect("couldn't find cargo manifest dir")
    );
    println!("cargo:rustc-env=ASSETS_PATH={assets_path}");

    let models_path = format!("{assets_path}/models");
    println!("cargo:rustc-env=MODELS_PATH={models_path}");
}
