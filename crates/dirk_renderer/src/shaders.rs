//! Compiled shader blobs used by the renderer.

use std::ffi::CStr;

/// A block of shader bytecode and the shader entry point name.
pub struct Shader {
    code: &'static [u8],
    entrypoint: &'static CStr,
}

impl Shader {
    /// Returns the shader code.
    #[must_use]
    pub const fn code(&self) -> &[u8] {
        self.code
    }

    /// Returns the shader code as little-endian `u32` words.
    ///
    /// # Panics
    ///
    /// Panics if the SPIR-V code size is not a multiple of 4 bytes.
    #[must_use]
    pub fn code_as_u32(&self) -> Vec<u32> {
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

/// Vertex shader.
pub const VERT: Shader = shader!("main_vs", c"main_vs");

/// Fragment shader.
pub const FRAG: Shader = shader!("main_fs", c"main_fs");
