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

// const ASSETS_PATH: &str = env!("ASSETS_PATH");
const MODELS_PATH: &str = env!("MODELS_PATH");

#[derive(derive_getters::Getters)]
pub struct Model {
    meshes: Vec<Mesh>,
    textures: Vec<Texture>,
    materials: Vec<Material>,
}

#[derive(derive_getters::Getters)]
pub struct Mesh {
    name: String,
    primitives: Vec<Primitive>,
}

#[derive(derive_getters::Getters)]
pub struct Primitive {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    texcoords: Vec<[f32; 2]>,
    indices: Vec<u32>,
    /// An optional index into the model's materials array.
    material: Option<usize>,
}

#[derive(derive_getters::Getters)]
pub struct Texture {
    name: String,
    /// Vector of RGBA8 pixels
    pixels: Vec<u8>,
    /// Width in pixels
    width: u32,
    /// Height in pixels
    height: u32,
}

#[derive(derive_getters::Getters)]
pub struct Material {
    /// An optional index into the model's textures array.
    base_color_texture: Option<usize>,
    /// An optional index into the model's textures array.
    metallic_roughness_texture: Option<usize>,
    /// An optional index into the model's textures array.
    normal_texture: Option<usize>,
    /// An optional index into the model's textures array.
    occlusion_texture: Option<usize>,
    /// An optional index into the model's textures array.
    emissive_texture: Option<usize>,
}

/// This is the main struct that handles loading resources.
pub struct ResourceManager {}

impl ResourceManager {
    pub fn load_model(name: &str) -> Result<Model> {
        let (gltf, buffers, images) = gltf::import(Self::model_path(name))?;

        let textures = images
            .iter()
            .enumerate()
            .map(|(i, image)| Self::load_texture(image, gltf.images().nth(i)))
            .collect();

        let materials = gltf.materials().map(Self::load_material).collect();

        let meshes = gltf
            .meshes()
            .map(|mesh| Mesh {
                name: mesh.name().unwrap_or("Nameless").to_string(),
                primitives: mesh
                    .primitives()
                    .map(|primitive| Self::load_primitive(&buffers, primitive))
                    .collect(),
            })
            .collect();

        Ok(Model {
            meshes,
            textures,
            materials,
        })
    }
    fn model_path(name: &str) -> String {
        format!("{}/{}/{}.gltf", MODELS_PATH, name, name)
    }
    fn load_texture(image: &gltf::image::Data, info: Option<gltf::Image>) -> Texture {
        let pixels = match image.format {
            gltf::image::Format::R8G8B8 => image
                .pixels
                .chunks_exact(3)
                .flat_map(|rgb| [rgb[0], rgb[1], rgb[2], 255])
                .collect(),
            gltf::image::Format::R8G8B8A8 => image.pixels.clone(),
            fmt => panic!("Unsuported glTF image format: {fmt:?}"),
        };

        let name = if let Some(info) = info {
            info.name().unwrap_or("Nameless")
        } else {
            "Nameless"
        }
        .to_string();

        Texture {
            name,
            pixels,
            width: image.width,
            height: image.height,
        }
    }
    fn load_material(mat: gltf::Material) -> Material {
        let pbr = mat.pbr_metallic_roughness();

        Material {
            base_color_texture: pbr
                .base_color_texture()
                .map(|t| t.texture().source().index()),
            metallic_roughness_texture: pbr
                .metallic_roughness_texture()
                .map(|t| t.texture().source().index()),
            normal_texture: mat.normal_texture().map(|t| t.texture().source().index()),
            occlusion_texture: mat
                .occlusion_texture()
                .map(|t| t.texture().source().index()),
            emissive_texture: mat.emissive_texture().map(|t| t.texture().source().index()),
        }
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
            material: primitive.material().index(),
        }
    }
}
