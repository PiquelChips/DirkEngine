//! This crate exports all the actual compiled shader binary blobs
//!
//! TODO: add documentation
#![allow(missing_docs)]

use crate::Shader;

pub const VERT: Shader = shader!("shader.vert", c"main");
pub const FRAG: Shader = shader!("shader.frag", c"main");
