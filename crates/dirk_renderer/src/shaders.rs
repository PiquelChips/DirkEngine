//! Compiled shader blobs used by the renderer.

use std::ffi::CStr;

use crate::{
    Result,
    resources::{
        descriptors::{
            layouts::SetLayout,
            sets::{MaterialSet, ObjectSet, SceneSet},
        },
        device::{Garbage, RenderDevice},
    },
    shaders::metadata::VertexInputLayout,
};
use ash::vk;

macro_rules! shader_code {
    ($name:literal) => {
        ShaderCode {
            code: (include_bytes!(concat!(env!("OUT_DIR"), "/", $name, ".spv"))),
        }
    };
}

/// A block of shader bytecode and the shader entry point name.
struct ShaderCode {
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

pub mod metadata;

pub struct VertexShader {
    code: ShaderCode,
    entrypoint: &'static CStr,
    // TODO: when creating the pipeline, compare these against
    // the layouts in GraphicsPipelineInfo. Return error if needed &
    // use to determine bindings & locations
    input_layouts: Vec<VertexInputLayout>,
    set_layouts: &'static [&'static [vk::DescriptorSetLayoutBinding<'static>]],
}

impl VertexShader {
    pub fn shader_create_info(
        &self,
        device: &mut RenderDevice,
    ) -> Result<vk::PipelineShaderStageCreateInfo<'_>> {
        shader_create_info(
            device,
            self.entrypoint,
            &self.code,
            vk::ShaderStageFlags::VERTEX,
        )
    }

    pub fn set_layouts(&self, device: &mut RenderDevice) -> Result<Vec<vk::DescriptorSetLayout>> {
        set_layouts(self.set_layouts, device)
    }
}

pub struct FragmentShader {
    code: ShaderCode,
    entrypoint: &'static CStr,
    set_layouts: &'static [&'static [vk::DescriptorSetLayoutBinding<'static>]],
}

impl FragmentShader {
    pub fn shader_create_info(
        &self,
        device: &mut RenderDevice,
    ) -> Result<vk::PipelineShaderStageCreateInfo<'_>> {
        shader_create_info(
            device,
            self.entrypoint,
            &self.code,
            vk::ShaderStageFlags::FRAGMENT,
        )
    }

    pub fn set_layouts(&self, device: &mut RenderDevice) -> Result<Vec<vk::DescriptorSetLayout>> {
        set_layouts(self.set_layouts, device)
    }
}

fn shader_create_info<'a>(
    device: &mut RenderDevice,
    entrypoint: &'a CStr,
    code: &'a ShaderCode,
    stage: vk::ShaderStageFlags,
) -> Result<vk::PipelineShaderStageCreateInfo<'a>> {
    let module = code.create_module(&device.device)?;
    device.destroy(Garbage::Shader(module));
    Ok(vk::PipelineShaderStageCreateInfo::default()
        .stage(stage)
        .module(module)
        .name(entrypoint))
}

fn set_layouts(
    set_layouts: &[&[vk::DescriptorSetLayoutBinding]],
    device: &mut RenderDevice,
) -> Result<Vec<vk::DescriptorSetLayout>> {
    set_layouts
        .iter()
        .map(|&set| {
            let info = vk::DescriptorSetLayoutCreateInfo::default().bindings(set);
            let layout = unsafe { device.device.create_descriptor_set_layout(&info, None)? };
            device.destroy(Garbage::DescriptorSetLayout(layout));
            Ok(layout)
        })
        .collect::<Result<Vec<_>>>()
}

/// Vertex shader.
pub const VERT: VertexShader = VertexShader {
    code: shader_code!("main_vs"),
    set_layouts: &[
        SceneSet::BINDINGS,
        ObjectSet::BINDINGS,
        MaterialSet::BINDINGS,
    ],
    input_layouts: Vec::new(), // TODO: populate
    entrypoint: c"main_vs",
};

/// Fragment shader.
pub const FRAG: FragmentShader = FragmentShader {
    code: shader_code!("main_fs"),
    entrypoint: c"main_fs",
    set_layouts: &[&[]], // TODO: populate
};
