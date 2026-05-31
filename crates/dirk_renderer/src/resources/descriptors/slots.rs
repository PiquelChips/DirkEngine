//! Single source of truth for descriptor set slot assignments.
//!
//! Every number here must match the corresponding `layout(set=N, binding=M)`
//! annotation in the GLSL shaders.

use ash::vk;

/// Pipeline set slot for the per-scene uniform buffer.
pub const SCENE_SET: u32 = 0;
/// Pipeline set slot for the per-scene uniform buffer as an array index.
pub const SCENE_SET_INDEX: usize = SCENE_SET as usize;
/// Binding index for scene data within the scene set.
pub const SCENE_BINDING: u32 = 0;
/// Shader stages that read the scene set.
pub const SCENE_STAGE: vk::ShaderStageFlags = vk::ShaderStageFlags::VERTEX;

/// Pipeline set slot for the per-object uniform buffer.
pub const OBJECT_SET: u32 = 1;
/// Pipeline set slot for the per-object uniform buffer as an array index.
pub const OBJECT_SET_INDEX: usize = OBJECT_SET as usize;
/// Binding index for object data within the object set.
pub const OBJECT_BINDING: u32 = 1;
/// Shader stages that read the object set.
pub const OBJECT_STAGE: vk::ShaderStageFlags = vk::ShaderStageFlags::VERTEX;

/// Pipeline set slot for the per-material combined image sampler.
pub const MATERIAL_SET: u32 = 2;
/// Pipeline set slot for the per-material sampler as an array index.
pub const MATERIAL_SET_INDEX: usize = MATERIAL_SET as usize;
/// Binding index for material data within the material set.
pub const MATERIAL_BINDING: u32 = 2;
/// Shader stages that read the material set.
pub const MATERIAL_STAGE: vk::ShaderStageFlags = vk::ShaderStageFlags::FRAGMENT;
