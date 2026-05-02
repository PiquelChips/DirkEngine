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
    #[must_use]
    pub const fn code(&self) -> &[u8] {
        self.code
    }
    /// Returns the code but in blocks of u32
    ///
    /// # Panics
    ///
    /// If the SPIR-V code size is not a multiple of
    /// 4. This would be invalid SPIR-V
    #[must_use]
    pub fn code_as_u32(&self) -> Vec<u32> {
        assert!(
            self.code.len().is_multiple_of(4),
            "SPIR-V size must be a multiple of 4"
        );
        self.code
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes(c.try_into().expect("4 byte chunks")))
            .collect()
    }
    /// Returns the entrypoint of this shader
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

mod blobs;
pub use blobs::*;
