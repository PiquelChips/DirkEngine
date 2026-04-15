use std::collections::HashMap;

use ash::vk;

use crate::{
    Result,
    resources::{
        buffer::{IndexBuffer, VertexBuffer},
        device::{Garbage, RenderDevice},
        image::Image,
    },
    utils::Vertex,
};

/// Complete GPU model.
pub struct Model {
    pub name: String,
    pub primitives: Vec<Primitive>,
    pub textures: Vec<Texture>,
    pub materials: Vec<resource_manager::Material>,
    /// One descriptor set per entry in `materials`.
    /// `vk::DescriptorSet::null()` if the material has no base-colour texture.
    pub material_sets: Vec<vk::DescriptorSet>,
}

/// All GPU-side handles for a single texture.
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

/// GPU-side handles for a single glTF primitive.
pub struct Primitive {
    pub vertex_buffer: VertexBuffer,
    pub index_buffer: IndexBuffer,
    pub index_count: u32,
    pub material: Option<usize>,
}

/// TODO: also find a way to do this dynamically
const MAX_MATERIAL_DESCRIPTOR_SET: u32 = 256;

pub struct AssetManager {
    device: RenderDevice,
    models: HashMap<String, Model>,

    material_descriptor_pool: vk::DescriptorPool,
}

impl AssetManager {
    pub fn new(device: &RenderDevice) -> Result<Self> {
        // MATERIAL DESCRIPTOR SETS
        let material_descriptor_pool = {
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
            models: HashMap::new(),
            material_descriptor_pool,
        })
    }
    pub fn get_or_upload_model(&mut self, name: &str) -> Result<&Model> {
        if self.models.contains_key(name) {
            return Ok(self.models.get(name).unwrap());
        }

        self.upload_model(resource_manager::ResourceManager::load_model(name)?)
    }
    pub fn upload_model(&mut self, model: resource_manager::Model) -> Result<&Model> {
        let primitives = model
            .meshes()
            .iter()
            .flat_map(|m| m.primitives().iter())
            .map(|p| self.upload_primitive(p))
            .collect::<Result<_>>()?;

        let textures: Vec<_> = model
            .textures()
            .iter()
            .map(|t| Image::upload_texture(self.device.clone(), t))
            .collect::<Result<_>>()?;

        let material_count = model.materials().len();
        // Allocate one set per material
        let layouts: Vec<vk::DescriptorSetLayout> =
            vec![self.device.layouts.material; material_count];

        let material_sets: Vec<vk::DescriptorSet> = if material_count > 0 {
            let alloc_info = vk::DescriptorSetAllocateInfo::default()
                .descriptor_pool(self.material_descriptor_pool)
                .set_layouts(&layouts);

            unsafe { self.device.device.allocate_descriptor_sets(&alloc_info)? }
        } else {
            Vec::new()
        };

        // Write the base-colour sampler into each set that has one
        for (i, mat) in model.materials().iter().enumerate() {
            let Some(&tex_idx) = mat.base_color_texture().as_ref() else {
                continue; // leave this set in its default (null) state
            };

            let tex = &textures[tex_idx];

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
        }

        self.models.insert(
            model.name().to_string(),
            Model {
                name: model.name().to_owned(),
                primitives,
                textures,
                materials: model.materials().to_vec(),
                material_sets,
            },
        );
        Ok(self.models.get(model.name()).unwrap())
    }

    fn upload_primitive(&mut self, prim: &resource_manager::Primitive) -> Result<Primitive> {
        let vertices: Vec<Vertex> = prim
            .positions()
            .iter()
            .enumerate()
            .map(|(i, &position)| Vertex {
                position,
                normal: prim.normals().get(i).copied().unwrap_or([0.0, 1.0, 0.0]),
                texcoord: prim.texcoords().get(i).copied().unwrap_or([0.0, 0.0]),
            })
            .collect();

        let vertex_buffer = VertexBuffer::upload_slice(self.device.clone(), &vertices)?;
        let index_buffer = IndexBuffer::upload_slice(self.device.clone(), prim.indices())?;

        Ok(Primitive {
            vertex_buffer,
            index_buffer,
            index_count: prim.indices().len() as u32,
            material: *prim.material(),
        })
    }
}
