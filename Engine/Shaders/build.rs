use std::{path::PathBuf, process::Command};

fn main() {
    let shader_dir = PathBuf::from("shaders");
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    let shaders = ["shader.vert", "shader.frag"];

    for shader in &shaders {
        let src_path = shader_dir.join(shader);
        let spv_path = out_dir.join(format!("{}.spv", shader));

        // Tell Cargo to re-run if the shader changes
        println!("cargo:rerun-if-changed={}", src_path.display());

        let status = Command::new("glslc")
            .args([src_path, "-o".into(), spv_path])
            .status()
            .expect("Failed to run glslc — is it installed and on PATH?");

        assert!(status.success(), "glslc failed for {}", shader);
    }
}
