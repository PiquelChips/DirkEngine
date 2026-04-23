#![allow(missing_docs)]

use shaderc::{CompileOptions, Compiler, ShaderKind};
use std::{fs, path::PathBuf};

fn main() {
    let shader_dir = PathBuf::from("shaders");
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    let compiler = Compiler::new().expect("Failed to init shaderc");
    let options = CompileOptions::new().unwrap();

    let shaders = [
        ("shader.vert", ShaderKind::Vertex),
        ("shader.frag", ShaderKind::Fragment),
    ];

    for (filename, kind) in &shaders {
        let src_path = shader_dir.join(filename);
        let spv_path = out_dir.join(format!("{}.spv", filename));

        // Tell Cargo to re-run if the shader changes
        println!("cargo:rerun-if-changed={}", src_path.display());

        let src = fs::read_to_string(&src_path)
            .unwrap_or_else(|_| panic!("Could not read {}", src_path.display()));

        let artifact = compiler
            .compile_into_spirv(&src, *kind, filename, "main", Some(&options))
            .unwrap_or_else(|e| panic!("Shader compile error in {}: {}", filename, e));

        fs::write(&spv_path, artifact.as_binary_u8()).expect("Failed to write .spv file");
    }
}
