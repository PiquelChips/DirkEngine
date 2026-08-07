//! Configures development-build Vulkan validation.

fn main() {
    println!("cargo:rustc-check-cfg=cfg(validation)");

    if std::env::var("PROFILE").is_ok_and(|profile| profile != "release") {
        println!("cargo:rustc-cfg=validation");
    }
}
