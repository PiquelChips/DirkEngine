//! This module contains all the logic necessary to rendering models.
//! As models are complex & have funny inter-dependencies, this is a
//! centralised system that has all textures, meshes, materials, ...
//!
//! When someone needs to render a model to the screen, all they have to do
//! is call [`ModelRegistry::render`] with their asset handle & a command buffer.
//! We handle the rest.

use std::{collections::HashMap, marker::PhantomData};

use ash::vk;

use crate::{
    Result,
    resources::{
        buffer::{IndexBuffer, VertexBuffer},
        command_pool::CommandBuffer,
        device::{Garbage, RenderDevice},
        image::Image,
    },
    utils::Vertex,
};

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Handle<T> {
    id: u32,
    _marker: PhantomData<T>,
}

impl<T> Copy for Handle<T> {}
impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        *self
    }
}

pub struct Texture {
    pub device: RenderDevice,
    pub image: Image,
    pub sampler: vk::Sampler,
    #[allow(unused)]
    pub mip_levels: u32,
}

impl Drop for Texture {
    fn drop(&mut self) {
        self.device.destroy(Garbage::Sampler(self.sampler));
    }
}

pub struct Primitive {
    pub vertex_buffer: VertexBuffer,
    pub index_buffer: IndexBuffer,
    pub index_count: u32,
    pub material_handle: Option<Handle<Material>>,
}

pub struct Mesh {
    pub primitives: Vec<Primitive>,
}

pub struct Material {
    #[allow(unused)]
    pub base_color: Handle<Texture>,
    pub descriptor_set: vk::DescriptorSet,
}

pub struct Model {
    // TODO: store transform with each mesh handle
    pub mesh_instances: Vec<Handle<Mesh>>,
}

/// TODO: better storage type for assets
/// look into generation arena or slotmap
struct AssetStorage<T> {
    assets: Vec<Option<T>>,
}

impl<T> AssetStorage<T> {
    fn new() -> Self {
        Self { assets: Vec::new() }
    }
    fn insert(&mut self, asset: T) -> Handle<T> {
        #[allow(clippy::cast_possible_truncation)]
        let id = self.assets.len() as u32;
        self.assets.push(Some(asset));
        Handle {
            id,
            _marker: PhantomData,
        }
    }
    fn get(&self, handle: Handle<T>) -> &T {
        self.assets[handle.id as usize]
            .as_ref()
            .expect("Invalid Handle")
    }
}

pub struct ModelRegistry {
    device: RenderDevice,

    textures: AssetStorage<Texture>,
    meshes: AssetStorage<Mesh>,
    materials: AssetStorage<Material>,
    models: HashMap<assets::AssetHandle, Model>,

    material_pool: vk::DescriptorPool,

    asset_load_consumer: events::Consumer<::assets::AssetLoaded<::assets::Model>>,
    asset_unload_consumer: events::Consumer<::assets::AssetUnloaded>,
}

/// TODO: descriptor pool
const MAX_MATERIAL_DESCRIPTOR_SET: u32 = 256;

impl ModelRegistry {
    pub fn new(device: &RenderDevice, events: &events::EventManager) -> Result<Self> {
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
            device: device.clone(),
            textures: AssetStorage::new(),
            meshes: AssetStorage::new(),
            materials: AssetStorage::new(),
            models: HashMap::new(),
            material_pool,

            asset_load_consumer: events.subscribe(),
            asset_unload_consumer: events.subscribe(),
        })
    }
    pub fn tick(&mut self) -> Result<()> {
        let events = self.asset_load_consumer.consume_all().collect::<Vec<_>>();
        for event in events {
            self.load_model(&event.handle)?;
        }

        let events = self.asset_unload_consumer.consume_all().collect::<Vec<_>>();
        for event in events {
            self.models.remove(&event.handle);
        }
        Ok(())
    }
    pub fn render_model(
        &self,
        handle: &assets::AssetHandle,
        cmd: &CommandBuffer,
        scene_set: vk::DescriptorSet,
        proxy_set: vk::DescriptorSet,
        pipeline_layout: vk::PipelineLayout,
    ) -> assets::Result<()> {
        let mut descriptor_sets = [scene_set, proxy_set, vk::DescriptorSet::null()];

        if handle.asset_type() != assets::AssetType::Model {
            return Err(assets::Error::TypeMismatch(handle.to_string()));
        }

        let model = self
            .models
            .get(handle)
            .ok_or(assets::Error::NotFound(handle.to_string()))?;
        // TODO: render all the meshes
        let mesh_handle = &model.mesh_instances[0];
        let mesh = self.meshes.get(*mesh_handle);

        for prim in &mesh.primitives {
            let mat_set = prim
                .material_handle
                .map_or(vk::DescriptorSet::null(), |mat| {
                    self.materials.get(mat).descriptor_set
                });

            descriptor_sets[2] = mat_set;

            unsafe {
                self.device.device.cmd_bind_descriptor_sets(
                    cmd.raw(),
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline_layout,
                    0,
                    &descriptor_sets,
                    &[],
                );
                self.device.device.cmd_bind_vertex_buffers(
                    cmd.raw(),
                    0,
                    &[prim.vertex_buffer.buffer()],
                    &[0],
                );
                self.device.device.cmd_bind_index_buffer(
                    cmd.raw(),
                    prim.index_buffer.buffer(),
                    0,
                    vk::IndexType::UINT32,
                );
                self.device
                    .device
                    .cmd_draw_indexed(cmd.raw(), prim.index_count, 1, 0, 0, 0);
            }
        }
        Ok(())
    }

    fn load_model(&mut self, handle: &assets::Handle<assets::Model>) -> Result<()> {
        let assets::Model {
            gltf,
            buffers,
            images,
        } = handle.get()?;

        let texture_handles = images
            .iter()
            .map(|image| {
                let tex = Image::upload_texture(&self.device, image)?;
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
                    .map(|prim| self.upload_primitive(&prim, &buffers, &material_handles))
                    .collect::<Result<Vec<_>>>()?;
                Ok(self.meshes.insert(Mesh { primitives }))
            })
            .collect::<Result<Vec<_>>>()?;

        self.models
            .insert(handle.handle(), Model { mesh_instances });
        Ok(())
    }

    fn create_materials(
        &mut self,
        materials: Vec<gltf::Material>,
        texture_refs: &[Handle<Texture>],
    ) -> Result<Vec<Handle<Material>>> {
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
                #[allow(clippy::unwrap_used)]
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
        primitive: &gltf::Primitive,
        buffers: &[gltf::buffer::Data],
        mat_refs: &[Handle<Material>],
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

        let vertex_buffer = VertexBuffer::upload_slice(&self.device, &vertices)?;
        let index_buffer = IndexBuffer::upload_slice(&self.device, &indices)?;

        // indices.len() will not surpass u32::MAX
        #[allow(clippy::cast_possible_truncation)]
        Ok(Primitive {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
            material_handle: primitive.material().index().map(|idx| mat_refs[idx]),
        })
    }
}
