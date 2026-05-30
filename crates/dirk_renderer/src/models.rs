//! This module contains all the logic necessary to rendering models.
//! As models are complex & have funny inter-dependencies, this is a
//! centralised system that has all textures, meshes, materials, ...
//!
//! When someone needs to render a model to the screen, all they have to do
//! is call [`ModelRegistry::render`] with their asset handle & a command buffer.
//! We handle the rest.

use std::{collections::HashMap, marker::PhantomData, ops::Deref};

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
struct Handle<T> {
    key: slotmap::DefaultKey,
    _marker: PhantomData<T>,
}

impl<T> Handle<T> {
    fn new(key: slotmap::DefaultKey) -> Self {
        Self {
            key,
            _marker: PhantomData,
        }
    }
}

impl<T> Copy for Handle<T> {}
impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Deref for Handle<T> {
    type Target = slotmap::DefaultKey;
    fn deref(&self) -> &Self::Target {
        &self.key
    }
}

pub struct Texture {
    pub device: RenderDevice,
    pub image: Image,
    pub sampler: vk::Sampler,
}

impl Drop for Texture {
    fn drop(&mut self) {
        self.device.destroy(Garbage::Sampler(self.sampler));
    }
}

struct Primitive {
    pub vertex_buffer: VertexBuffer,
    pub index_buffer: IndexBuffer,
    pub index_count: u32,
    pub material_handle: Option<Handle<Material>>,
}

struct Mesh {
    pub primitives: Vec<Primitive>,
}

struct Material {
    #[allow(unused)]
    pub base_color: Handle<Texture>,
    pub descriptor_set: vk::DescriptorSet,
}

struct Model {
    // TODO: store transform with each mesh handle
    pub meshes: Vec<Handle<Mesh>>,
}

pub struct ModelRegistry {
    device: RenderDevice,

    textures: slotmap::SlotMap<slotmap::DefaultKey, Texture>,
    meshes: slotmap::SlotMap<slotmap::DefaultKey, Mesh>,
    materials: slotmap::SlotMap<slotmap::DefaultKey, Material>,
    models: HashMap<dirk_assets::AssetHandle, Model>,

    material_pool: vk::DescriptorPool,

    asset_load_consumer: dirk_events::Consumer<::dirk_assets::AssetLoaded<::dirk_assets::Model>>,
    asset_unload_consumer: dirk_events::Consumer<::dirk_assets::AssetUnloaded>,
}

/// TODO: descriptor pool
const MAX_MATERIAL_DESCRIPTOR_SET: u32 = 256;

impl ModelRegistry {
    pub fn new(device: &RenderDevice, events: &dirk_events::EventManager) -> Result<Self> {
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
            textures: slotmap::SlotMap::new(),
            meshes: slotmap::SlotMap::new(),
            materials: slotmap::SlotMap::new(),
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
        handle: &dirk_assets::AssetHandle,
        cmd: &CommandBuffer,
        scene_set: vk::DescriptorSet,
        proxy_set: vk::DescriptorSet,
        pipeline_layout: vk::PipelineLayout,
    ) -> dirk_assets::Result<()> {
        let mut descriptor_sets = [scene_set, proxy_set, vk::DescriptorSet::null()];

        if handle.asset_type() != dirk_assets::AssetType::Model {
            return Err(dirk_assets::Error::TypeMismatch(handle.to_string()));
        }

        let model = self
            .models
            .get(handle)
            .ok_or(dirk_assets::Error::NotFound(handle.to_string()))?;

        let primitives = model
            .meshes
            .iter()
            .flat_map(|&mesh| self.meshes[*mesh].primitives.iter());

        for prim in primitives {
            let mat_set = prim
                .material_handle
                .map_or(vk::DescriptorSet::null(), |mat| {
                    self.materials[*mat].descriptor_set
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

    fn load_model(&mut self, handle: &dirk_assets::Handle<dirk_assets::Model>) -> Result<()> {
        let dirk_assets::Model {
            gltf,
            buffers,
            images,
        } = handle.get()?;

        let texture_handles = images
            .iter()
            .map(|image| {
                let tex = Image::upload_texture(&self.device, image)?;
                Ok(Handle::new(self.textures.insert(tex)))
            })
            .collect::<Result<Vec<_>>>()?;

        let material_handles =
            self.create_materials(gltf.materials().collect(), &texture_handles)?;

        let meshes = gltf
            .meshes()
            .map(|mesh| {
                let primitives = mesh
                    .primitives()
                    .map(|prim| self.upload_primitive(&prim, &buffers, &material_handles))
                    .collect::<Result<Vec<_>>>()?;
                Ok(Handle::new(self.meshes.insert(Mesh { primitives })))
            })
            .collect::<Result<Vec<_>>>()?;

        self.models.insert(handle.handle(), Model { meshes });
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
                let tex = &self.textures[*tex_handle];
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
                Handle::new(self.materials.insert(Material {
                    base_color: tex_handle,
                    descriptor_set: material_sets[i],
                }))
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

impl Drop for ModelRegistry {
    fn drop(&mut self) {
        self.textures.clear();
        self.meshes.clear();
        self.materials.clear();
        self.models.clear();
        unsafe {
            self.device
                .device
                .destroy_descriptor_pool(self.material_pool, None);
        };
    }
}
