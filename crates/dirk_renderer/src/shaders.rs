//! Compiled shader blobs used by the renderer.

use std::ffi::CStr;

use crate::{
    resources::descriptors::{
        layouts::SetLayout,
        sets::{MaterialSet, ObjectSet, SceneSet},
    },
    shaders::metadata::{FragmentShader, Shader, VertexInputLayout, VertexShader},
};
use ash::vk;

macro_rules! shader_code {
    ($name:literal) => {
        ShaderCode {
            code: (include_bytes!(concat!(env!("OUT_DIR"), "/", $name, ".spv"))),
        }
    };
}

pub mod metadata;

/// A block of shader bytecode and the shader entry point name.
pub(crate) struct ShaderCode {
    code: &'static [u8],
}

impl ShaderCode {
    fn create_module(&self, device: &ash::Device) -> crate::Result<ash::vk::ShaderModule> {
        let code = self.code_as_u32();
        let info = ash::vk::ShaderModuleCreateInfo::default().code(code.as_slice());
        Ok(unsafe { device.create_shader_module(&info, None)? })
    }

    /// Returns the shader code as little-endian `u32` words.
    ///
    /// # Panics
    ///
    /// Panics if the SPIR-V code size is not a multiple of 4 bytes.
    #[must_use]
    fn code_as_u32(&self) -> Vec<u32> {
        assert!(
            self.code.len().is_multiple_of(4),
            "SPIR-V size must be a multiple of 4"
        );
        self.code
            .chunks_exact(4)
            .map(|chunk| u32::from_le_bytes(chunk.try_into().expect("4 byte chunks")))
            .collect()
    }
}

/// Vertex shader.
pub struct MainVS;

impl Shader for MainVS {
    const CODE: ShaderCode = shader_code!("main_vs");
    const ENTRYPOINT: &'static CStr = c"main_vs";
    const SET_LAYOUTS: &'static [&'static [vk::DescriptorSetLayoutBinding<'static>]] = &[
        SceneSet::BINDINGS,
        ObjectSet::BINDINGS,
        MaterialSet::BINDINGS,
    ];
}

impl VertexShader for MainVS {
    const INPUT_LAYOUTS: Vec<VertexInputLayout> = Vec::new(); // TODO: populate
}

/// Fragment shader.
pub struct MainFS;

impl Shader for MainFS {
    const CODE: ShaderCode = shader_code!("main_fs");
    const ENTRYPOINT: &'static CStr = c"main_fs";
    const SET_LAYOUTS: &'static [&'static [vk::DescriptorSetLayoutBinding<'static>]] = &[];
}

impl FragmentShader for MainFS {}
