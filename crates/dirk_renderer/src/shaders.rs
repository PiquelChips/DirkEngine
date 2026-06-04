//! Compiled shader blobs used by the renderer.

use std::ffi::CStr;

use crate::shaders::{metadata::VertexInputLayout, private::ShaderPrivate};

mod private {
    use crate::shaders::ShaderCode;

    pub trait ShaderPrivate {
        fn shader_code(&self) -> &ShaderCode;
    }
}

pub mod metadata;

pub trait Shader: ShaderPrivate {
    /// Returns the shader code.
    #[must_use]
    #[allow(unused)]
    fn code(&self) -> &[u8] {
        self.shader_code().code()
    }

    /// Returns the shader code as little-endian `u32` words.
    ///
    /// # Panics
    ///
    /// Panics if the SPIR-V code size is not a multiple of 4 bytes.
    #[must_use]
    fn code_as_u32(&self) -> Vec<u32> {
        self.shader_code().code_as_u32()
    }

    /// Returns the shader entry point.
    #[must_use]
    fn entrypoint(&self) -> &CStr {
        self.shader_code().entrypoint()
    }
}

impl<T: ShaderPrivate> Shader for T {}

/// A block of shader bytecode and the shader entry point name.
pub(crate) struct ShaderCode {
    code: &'static [u8],
    entrypoint: &'static CStr,
}

impl ShaderPrivate for ShaderCode {
    fn shader_code(&self) -> &ShaderCode {
        self
    }
}

impl ShaderCode {
    /// Returns the shader code.
    #[must_use]
    const fn code(&self) -> &[u8] {
        self.code
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

    /// Returns the shader entry point.
    #[must_use]
    const fn entrypoint(&self) -> &CStr {
        self.entrypoint
    }
}

macro_rules! shader {
    ($name:literal, $entrypoint:literal) => {
        ShaderCode {
            code: include_bytes!(concat!(env!("OUT_DIR"), "/", $name, ".spv")),
            entrypoint: $entrypoint,
        }
    };
}

pub struct VertexShader {
    code: ShaderCode,
    // TODO: when creating the pipeline, compare these against
    // the layouts in GraphicsPipelineInfo. Return error if needed &
    // use to determine bindings & locations
    inputs: Vec<VertexInputLayout>,
}

impl ShaderPrivate for VertexShader {
    fn shader_code(&self) -> &ShaderCode {
        &self.code
    }
}

pub struct FragmentShader(ShaderCode);

impl ShaderPrivate for FragmentShader {
    fn shader_code(&self) -> &ShaderCode {
        &self.0
    }
}

/// Vertex shader.
pub const VERT: VertexShader = VertexShader {
    code: shader!("main_vs", c"main_vs"),
    inputs: Vec::new(), // TODO: populate
};

/// Fragment shader.
pub const FRAG: FragmentShader = FragmentShader(shader!("main_fs", c"main_fs"));
