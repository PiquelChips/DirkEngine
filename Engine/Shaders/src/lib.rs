//! This crate contains all the shaders used in the engine.

use std::ffi::CStr;

/// A simple struct that holds a block of shader bytecode and
/// the name of the shader's entrypoint.
pub struct Shader {
    code: &'static [u8],
    entrypoint: &'static CStr,
}

impl Shader {
    /// Returns the shader code
    pub const fn code(&self) -> &[u8] {
        self.code
    }
    /// Returns the code but in blocks of u32
    pub fn code_as_u32(&self) -> Vec<u32> {
        assert!(
            self.code.len().is_multiple_of(4),
            "SPIR-V size must be a multiple of 4"
        );
        self.code
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
            .collect()
    }
    /// Returns the entrypoint of this shader
    pub const fn entrypoint(&self) -> &CStr {
        self.entrypoint
    }
}

macro_rules! shader {
    ($name:literal, $entrypoint:literal) => {
        Shader {
            code: include_bytes!(concat!(env!("OUT_DIR"), "/", $name, ".spv")),
            entrypoint: $entrypoint,
        }
    };
}

pub const VERT: Shader = shader!("shader.vert", c"main");
pub const FRAG: Shader = shader!("shader.frag", c"main");
