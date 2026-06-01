//! This crate exports all the actual compiled shader binary blobs
//!
//! TODO: add documentation
#![allow(missing_docs)]

use crate::Shader;

pub const VERT: Shader = shader!("main_vs", c"main_vs");
pub const FRAG: Shader = shader!("main_fs", c"main_fs");
