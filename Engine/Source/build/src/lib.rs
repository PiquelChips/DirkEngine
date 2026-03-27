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
