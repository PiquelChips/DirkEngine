#![allow(missing_docs)]

use anyhow::anyhow;
use cargo_gpu_install::{
    install::Install,
    spirv_builder::{ModuleResult, SpirvMetadata},
};
use std::{fs, path::PathBuf};

fn main() -> anyhow::Result<()> {
    dirk_build::configure_platform();

    println!("cargo:rustc-check-cfg=cfg(validation)");

    let profile = std::env::var("PROFILE").unwrap_or_default();
    if profile != "release" {
        println!("cargo:rustc-cfg=validation");
    }

    build_shaders()?;

    Ok(())
}

fn build_shaders() -> anyhow::Result<()> {
    let shader_crate = PathBuf::from("shaders");
    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);

    println!("cargo:rerun-if-changed=shaders/Cargo.toml");
    println!("cargo:rerun-if-changed=shaders/src/lib.rs");

    let backend = Install::from_shader_crate(shader_crate.clone()).run()?;
    let mut builder = backend.to_spirv_builder(shader_crate, "spirv-unknown-vulkan1.3");
    builder.build_script.defaults = true;
    builder.build_script.env_shader_spv_path = Some(false);
    builder.multimodule = true;
    builder.spirv_metadata = SpirvMetadata::None;

    let spv_result = builder.build()?;
    let ModuleResult::MultiModule(modules) = spv_result.module else {
        return Err(anyhow!("expected one SPIR-V module per shader entry point"));
    };
    for (entrypoint, source_path) in modules {
        fs::copy(source_path, out_dir.join(format!("{entrypoint}.spv")))?;
    }

    Ok(())
}
