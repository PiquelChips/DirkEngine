//! This crate has the ResourceManager struct.
//! This struct handles all loading of data from the
//! file system. It is intimately linked with the platform crate
//! for platform-specifc resource loading.
//!
//! This crate can load glTF models into internal structs for upload by
//! the renderer.
//! It can also load textures for use by the renderer.
//!
//! In the future, it will also load sound and other assets. However,
//! as these systems aren't implemented yet, the resource manager does
//! not support loading them.

mod errors;
pub use errors::Error;
use errors::Result;

const ASSETS_PATH: &str = env!("ASSETS_PATH");
const MODELS_PATH: &str = env!("MODELS_PATH");

pub struct Model {
    meshes: Vec<Mesh>,
}

pub struct Mesh {
    name: String,
    primitives: Vec<Primitive>,
}

pub struct Primitive {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    texcoords: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

/// This is the main struct that handles loading resources.
pub struct ResourceManager {}

impl ResourceManager {
    pub fn load_model(name: &str) -> Result<Model> {
        let (gltf, buffers, images) = gltf::import(Self::model_path(name))?;

        Ok(Model {
            meshes: gltf
                .meshes()
                .map(|mesh| Mesh {
                    name: mesh.name().unwrap_or_default().to_string(),
                    primitives: mesh
                        .primitives()
                        .map(|primitive| Self::load_primitive(&buffers, primitive))
                        .collect(),
                })
                .collect(),
        })
    }
    fn model_path(name: &str) -> String {
        format!("{}/{}/{}.gltf", MODELS_PATH, name, name)
    }
    fn load_primitive(buffers: &[gltf::buffer::Data], primitive: gltf::Primitive) -> Primitive {
        let reader = primitive.reader(|buf| Some(&buffers[buf.index()]));
        Primitive {
            positions: reader
                .read_positions()
                .map(Iterator::collect)
                .unwrap_or_default(),
            normals: reader
                .read_normals()
                .map(Iterator::collect)
                .unwrap_or_default(),
            texcoords: reader
                .read_tex_coords(0)
                .map(|iter| iter.into_f32().collect())
                .unwrap_or_default(),
            indices: reader
                .read_indices()
                .map(|iter| iter.into_u32().collect())
                .unwrap_or_default(),
        }
    }
}
