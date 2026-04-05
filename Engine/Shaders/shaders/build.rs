use cargo_gpu_install::install::Install;
use cargo_gpu_install::spirv_builder::SpirvMetadata;
use std::path::PathBuf;

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    // path to your shader crate
    let shader_crate = PathBuf::from("../main");

    // install the toolchain and build the `rustc_codegen_spirv` codegen backend with it
    let backend = Install::from_shader_crate(shader_crate.clone()).run()?;

    // build the shader crate
    let mut builder = backend.to_spirv_builder(shader_crate, "spirv-unknown-vulkan1.3");
    // set to true when you're in a build script, false when outside one
    builder.build_script.defaults = true;
    // enable debug information in the shaders
    builder.spirv_metadata = SpirvMetadata::Full;
    let spv_result = builder.build()?;
    let path_to_spv = spv_result.module.unwrap_single();

    // emit path to the artifact into env var, use it anywhere in your crate like:
    // > include_str!(env!("MY_SHADER_PATH"))
    println!("cargo::rustc-env=MY_SHADER_PATH={}", path_to_spv.display());

    // you could also generate some rust source code into the `std::env::var("OUT_DIR")` dir
    // and use `include!(concat!(env!("OUT_DIR"), "/shader_symbols.rs"));` to include it
    Ok(())
}
