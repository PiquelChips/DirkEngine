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

/// Internal representation of a model loaded from a gltf model.
#[derive(derive_getters::Getters)]
pub struct Model {
    meshes: Vec<Mesh>,
    textures: Vec<Texture>,
    materials: Vec<Material>,
}

/// Internal representation of a mesh. Has many primitives that each
/// can be rendered to make the full model.
#[derive(derive_getters::Getters)]
pub struct Mesh {
    name: String,
    primitives: Vec<Primitive>,
}

/// Internal primitive type. Stores all data required to render
/// the primitive: positions, normals, texcoords, indices & materials.
#[derive(derive_getters::Getters)]
pub struct Primitive {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    texcoords: Vec<[f32; 2]>,
    indices: Vec<u32>,
    /// An optional index into the model's materials array.
    material: Option<usize>,
}

/// A texture. Stores a name and size alongside the pixel data.
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

/// A material. Stores indices into the model's textures array.
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
/// Doesn't actually have any internal state, just a bunch
/// of utility functions for loading.
pub struct ResourceManager {}

impl ResourceManager {
    /// Loads a gltf model with the specified name.
    /// Will load it from `Engine/Assets/{name}/{name}.gltf`.
    /// Will return an error if the model is not in that location.
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
    /// Gives the path to a model from its name.
    /// Just a simple utility to get `MODELS_PATH/{name}/{name}.gltf`.
    fn model_path(name: &str) -> String {
        format!("{MODELS_PATH}/{name}/{name}.gltf")
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

#[cfg(test)]
mod tests {
    use super::*;

    // -- model_path --

    #[test]
    fn model_path_formats_correctly() {
        let path = ResourceManager::model_path("Sword");
        // Should end with the expected sub-path regardless of MODELS_PATH prefix
        assert!(
            path.ends_with("/Sword/Sword.gltf"),
            "unexpected path: {path}"
        );
    }

    #[test]
    fn model_path_contains_name_twice() {
        // The name must appear as both the directory and the file stem.
        let name = "PlayerShip";
        let path = ResourceManager::model_path(name);
        let occurrences = path.matches(name).count();
        assert_eq!(occurrences, 2, "name should appear twice in path: {path}");
    }

    // -- load_texture: pixel conversion --

    /// Helper: build a minimal gltf::image::Data for a 1×1 RGB pixel.
    fn rgb_image_data(r: u8, g: u8, b: u8) -> gltf::image::Data {
        gltf::image::Data {
            format: gltf::image::Format::R8G8B8,
            pixels: vec![r, g, b],
            width: 1,
            height: 1,
        }
    }

    /// Helper: build a minimal gltf::image::Data for a 1×1 RGBA pixel.
    fn rgba_image_data(r: u8, g: u8, b: u8, a: u8) -> gltf::image::Data {
        gltf::image::Data {
            format: gltf::image::Format::R8G8B8A8,
            pixels: vec![r, g, b, a],
            width: 1,
            height: 1,
        }
    }

    #[test]
    fn load_texture_rgb_injects_full_alpha() {
        let data = rgb_image_data(10, 20, 30);
        let tex = ResourceManager::load_texture(&data, None);

        assert_eq!(
            *tex.pixels(),
            vec![10, 20, 30, 255],
            "RGB→RGBA conversion should inject alpha = 255"
        );
    }

    #[test]
    fn load_texture_rgb_multi_pixel_layout() {
        // 2 pixels: [R1 G1 B1] [R2 G2 B2] → [R1 G1 B1 255 R2 G2 B2 255]
        let data = gltf::image::Data {
            format: gltf::image::Format::R8G8B8,
            pixels: vec![1, 2, 3, 4, 5, 6],
            width: 2,
            height: 1,
        };
        let tex = ResourceManager::load_texture(&data, None);
        assert_eq!(*tex.pixels(), vec![1, 2, 3, 255, 4, 5, 6, 255]);
    }

    #[test]
    fn load_texture_rgba_passthrough() {
        // RGBA pixels must be stored unchanged.
        let data = rgba_image_data(50, 100, 150, 200);
        let tex = ResourceManager::load_texture(&data, None);
        assert_eq!(*tex.pixels(), vec![50, 100, 150, 200]);
    }

    #[test]
    fn load_texture_rgba_preserves_partial_alpha() {
        // Make sure a semi-transparent pixel isn't accidentally modified.
        let data = rgba_image_data(255, 0, 128, 64);
        let tex = ResourceManager::load_texture(&data, None);
        assert_eq!(*tex.pixels(), vec![255, 0, 128, 64]);
    }

    #[test]
    fn load_texture_size_is_preserved() {
        let data = gltf::image::Data {
            format: gltf::image::Format::R8G8B8A8,
            pixels: vec![0u8; 4 * 3 * 7], // 3 × 7 image
            width: 3,
            height: 7,
        };
        let tex = ResourceManager::load_texture(&data, None);
        assert_eq!(*tex.width(), 3);
        assert_eq!(*tex.height(), 7);
    }

    #[test]
    fn load_texture_no_info_gives_nameless() {
        let data = rgb_image_data(0, 0, 0);
        let tex = ResourceManager::load_texture(&data, None);
        assert_eq!(tex.name(), "Nameless");
    }

    // -- load_texture: panic on unsupported format --

    #[test]
    #[should_panic(expected = "Unsuported glTF image format")]
    fn load_texture_panics_on_unsupported_format() {
        let data = gltf::image::Data {
            format: gltf::image::Format::R8, // not handled
            pixels: vec![128],
            width: 1,
            height: 1,
        };
        ResourceManager::load_texture(&data, None);
    }
}
