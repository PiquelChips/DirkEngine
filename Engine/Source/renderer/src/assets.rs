use ash::vk;
use std::collections::HashMap;
use std::marker::PhantomData;
use types::handle::{
    self, AssetHandle, AssetType, MaterialHandle, MeshHandle, ModelHandle, TextureHandle,
};

use crate::{
    Result,
    resources::{
        buffer::{IndexBuffer, VertexBuffer},
        device::{Garbage, RenderDevice},
        image::Image,
    },
    utils::Vertex,
};

pub struct Texture {
    pub device: RenderDevice,
    pub image: Image,
    pub sampler: vk::Sampler,
    pub mip_levels: u32,
}

impl Drop for Texture {
    fn drop(&mut self) {
        self.device.destroy(Garbage::Sampler(self.sampler));
    }
}

pub struct Primitive {
    vertex_buffer: VertexBuffer,
    index_buffer: IndexBuffer,
    index_count: u32,
    material_handle: Option<MaterialHandle>,
}

pub struct Mesh {
    primitives: Vec<Primitive>,
}

pub struct Material {
    base_color: TextureHandle,
    descriptor_set: vk::DescriptorSet,
}

pub struct Model {
    // TODO: store transform with each mesh handle
    pub mesh_instances: Vec<MeshHandle>,
}

/// TODO: better storage type for assets
struct AssetStorage<A: AssetType, T> {
    assets: Vec<Option<T>>,
    // TODO: future
    _assets: HashMap<AssetHandle<A>, T>,
}

impl<A: AssetType, T> AssetStorage<A, T> {
    fn new() -> Self {
        Self {
            assets: Vec::new(),
            _assets: HashMap::new(),
        }
    }
    fn insert(&mut self, asset: T) -> AssetHandle<A> {
        let id = self.assets.len() as u32;
        self.assets.push(Some(asset));
        AssetHandle {
            id,
            _marker: PhantomData,
        }
    }
    fn get(&self, handle: AssetHandle<A>) -> &T {
        self.assets[handle.id as usize]
            .as_ref()
            .expect("Invalid Handle")
    }
}

pub struct AssetManager {
    device: RenderDevice,

    textures: AssetStorage<handle::Texture, Texture>,
    meshes: AssetStorage<handle::Mesh, Mesh>,
    materials: AssetStorage<handle::Material, Material>,
    models: AssetStorage<handle::Model, Model>,

    path_to_model: HashMap<String, ModelHandle>,

    material_pool: vk::DescriptorPool,
}

/// TODO: also find a way to do this dynamically
const MAX_MATERIAL_DESCRIPTOR_SET: u32 = 256;

const MODELS_PATH: &str = env!("MODELS_PATH");

impl AssetManager {
    pub fn new(device: RenderDevice) -> Result<Self> {
        let material_pool = {
            let pool_size = vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: MAX_MATERIAL_DESCRIPTOR_SET,
            };
            let pool_info = vk::DescriptorPoolCreateInfo::default()
                .pool_sizes(std::slice::from_ref(&pool_size))
                .max_sets(MAX_MATERIAL_DESCRIPTOR_SET);

            unsafe { device.device.create_descriptor_pool(&pool_info, None)? }
        };

        Ok(Self {
            device,
            textures: AssetStorage::new(),
            meshes: AssetStorage::new(),
            materials: AssetStorage::new(),
            models: AssetStorage::new(),
            path_to_model: HashMap::new(),
            material_pool,
        })
    }

    pub fn load_model(&mut self, name: &str) -> Result<ModelHandle> {
        if let Some(handle) = self.path_to_model.get(name) {
            return Ok(*handle);
        }

        let (gltf, buffers, images) = gltf::import(Self::model_path(name))?;

        let texture_handles = images
            .iter()
            .map(|image| {
                let tex = Image::upload_texture(self.device.clone(), image)?;
                Ok(self.textures.insert(tex))
            })
            .collect::<Result<Vec<_>>>()?;

        let material_handles =
            self.create_materials(gltf.materials().collect(), &texture_handles)?;

        let mesh_instances = gltf
            .meshes()
            .map(|mesh| {
                let primitives = mesh
                    .primitives()
                    .map(|prim| Ok(self.upload_primitive(prim, &buffers, &material_handles)?))
                    .collect::<Result<Vec<_>>>()?;
                Ok(self.meshes.insert(Mesh { primitives }))
            })
            .collect::<Result<Vec<_>>>()?;

        let model_handle = self.models.insert(Model { mesh_instances });
        self.path_to_model.insert(name.to_string(), model_handle);
        Ok(model_handle)
    }

    fn create_materials(
        &mut self,
        materials: Vec<gltf::Material>,
        texture_refs: &[TextureHandle],
    ) -> Result<Vec<MaterialHandle>> {
        let material_count = materials.len();
        // Allocate one set per material
        let layouts: Vec<vk::DescriptorSetLayout> =
            vec![self.device.layouts.material; material_count];

        let material_sets: Vec<vk::DescriptorSet> = if material_count > 0 {
            let alloc_info = vk::DescriptorSetAllocateInfo::default()
                .descriptor_pool(self.material_pool)
                .set_layouts(&layouts);

            unsafe { self.device.device.allocate_descriptor_sets(&alloc_info)? }
        } else {
            Vec::new()
        };

        Ok(materials
            .into_iter()
            .enumerate()
            .map(|(i, mat)| {
                let pbr = mat.pbr_metallic_roughness();
                // TODO: actually PBR materials
                let tex = pbr.base_color_texture().unwrap().texture().source().index();

                let tex_handle = texture_refs[tex];
                let tex = self.textures.get(tex_handle);
                let image_info = vk::DescriptorImageInfo::default()
                    .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                    .image_view(tex.image.view())
                    .sampler(tex.sampler);

                let write = vk::WriteDescriptorSet::default()
                    .dst_set(material_sets[i])
                    .dst_binding(2) // matches layouts.material
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(std::slice::from_ref(&image_info));

                unsafe { self.device.device.update_descriptor_sets(&[write], &[]) };
                self.materials.insert(Material {
                    base_color: tex_handle,
                    descriptor_set: material_sets[i],
                })
            })
            .collect())
    }

    fn upload_primitive(
        &self,
        primitive: gltf::Primitive,
        buffers: &[gltf::buffer::Data],
        mat_refs: &[MaterialHandle],
    ) -> Result<Primitive> {
        let reader = primitive.reader(|buf| Some(&buffers[buf.index()]));

        let positions: Vec<_> = reader
            .read_positions()
            .map(Iterator::collect)
            .unwrap_or_default();
        let normals: Vec<_> = reader
            .read_normals()
            .map(Iterator::collect)
            .unwrap_or_default();
        let texcoords: Vec<_> = reader
            .read_tex_coords(0)
            .map(|iter| iter.into_f32().collect())
            .unwrap_or_default();
        let indices: Vec<_> = reader
            .read_indices()
            .map(|iter| iter.into_u32().collect())
            .unwrap_or_default();

        let vertices: Vec<Vertex> = positions
            .iter()
            .enumerate()
            .map(|(i, &position)| Vertex {
                position,
                normal: normals.get(i).copied().unwrap_or([0.0, 1.0, 0.0]),
                texcoord: texcoords.get(i).copied().unwrap_or([0.0, 0.0]),
            })
            .collect();

        let vertex_buffer = VertexBuffer::upload_slice(self.device.clone(), &vertices)?;
        let index_buffer = IndexBuffer::upload_slice(self.device.clone(), &indices)?;

        Ok(Primitive {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
            material_handle: primitive.material().index().map(|idx| mat_refs[idx]),
        })
    }

    pub fn get_mesh(&self, handle: MeshHandle) -> &Mesh {
        self.meshes.get(handle)
    }

    pub fn get_material(&self, handle: MaterialHandle) -> &Material {
        self.materials.get(handle)
    }

    /// Gives the path to a model from its name.
    /// Just a simple utility to get `MODELS_PATH/{name}/{name}.gltf`.
    fn model_path(name: &str) -> String {
        format!("{MODELS_PATH}/{name}/{name}.gltf")
    }
}
