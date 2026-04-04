//! This crate exports all the shaders. It's the only crate that
//! needs to be included by other engine modules

pub const MAIN: &[u8] = include_bytes!(env!("<shader_crate>.spv"));
