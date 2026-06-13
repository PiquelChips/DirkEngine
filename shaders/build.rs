#![allow(missing_docs)]

use cargo_gpu_install::{
    install::Install,
    spirv_builder::{ModuleResult, SpirvMetadata},
};
use std::{fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let shader_crate = PathBuf::from("shaders");
    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);

    println!("cargo:rerun-if-changed=shaders/Cargo.toml");
    // TODO: rerun on any rust file change
    println!("cargo:rerun-if-changed=shaders/src/lib.rs");

    let backend = Install::from_shader_crate(shader_crate.clone()).run()?;
    let mut builder = backend.to_spirv_builder(shader_crate, "spirv-unknown-vulkan1.3");
    builder.build_script.defaults = true;
    builder.build_script.env_shader_spv_path = Some(false);
    builder.multimodule = true;
    builder.spirv_metadata = SpirvMetadata::None;

    let spv_result = builder.build()?;
    let ModuleResult::MultiModule(modules) = spv_result.module else {
        return Err("expected one SPIR-V module per shader entry point".into());
    };
    for (entrypoint, source_path) in modules {
        fs::copy(source_path, out_dir.join(format!("{entrypoint}.spv")))?;
    }

    Ok(())
}
