//! This crate has a bunch of utilities for the engine build scripts.
//! This mainly avoids duplicating the code of stuff like platform
//! configuration and the editor.

use std::{collections::HashMap, path::PathBuf};

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

/// Returns the directory of the current cargo workspace
///
/// # Panics
///
/// Will panic if it fails to run `cargo metadata`
#[must_use]
pub fn workspace_dir() -> PathBuf {
    let metadata = cargo_metadata::MetadataCommand::new()
        .exec()
        .expect("failed to run cargo metadata");

    metadata.workspace_root.into_std_path_buf()
}

/// Will add usefull relative paths to the compilation env.
///
/// Paths can be added in the function body inserting them into the [`HashMap`]
pub fn setup_paths() {
    println!(
        "cargo:rustc-env=WORKSPACE_ROOT={}",
        workspace_dir().display()
    );

    let workspace_root = PathBuf::from(".");

    let mut paths = HashMap::new();
    paths.insert("SAVED_PATH", "saved");
    paths.insert("ASSETS_PATH", "assets");

    for (name, path) in paths {
        println!(
            "cargo:rustc-env={name}={}",
            workspace_root.join(path).display()
        );
    }
}
