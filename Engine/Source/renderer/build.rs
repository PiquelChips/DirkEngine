#![allow(missing_docs)]

fn main() {
    build::configure_platform();

    println!("cargo:rustc-check-cfg=cfg(validation)");

    let profile = std::env::var("PROFILE").unwrap_or_default();
    if profile != "release" {
        println!("cargo:rustc-cfg=validation");
    }
}
