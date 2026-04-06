//! This crate contains all the shaders used in the engine.

pub struct Shader {
    code: &'static [u8],
    entrypoint: &'static str,
}

impl Shader {
    pub const fn code(&self) -> &[u8] {
        self.code
    }
    pub const fn entrypoint(&self) -> &str {
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

pub const VERT: Shader = shader!("shader.vert", "main");
pub const FRAG: Shader = shader!("shader.frag", "main");
