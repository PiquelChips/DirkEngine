//! This module holds proxies for various engine objects

mod types;
use std::collections::HashMap;

use ash::vk;
use gpu_allocator::MemoryLocation;
use universe::{Entity, WorldId};

use crate::{
    MAX_FRAMES_IN_FLIGHT, MAX_RENDERABLES, Result,
    pipeline::GraphicsPipeline,
    proxy::scene::Scene,
    resources::{
        device::RenderDevice,
        image::{Image, ImageCreateInfo},
    },
};

mod scene;
pub mod systems;

// TODO: shouldn't be public
pub use types::*;

/// This is the renderer proxy for the [`Universe`]. It also has
/// most of the rendering state needed to render each scene.
pub struct SceneManager {
    device: RenderDevice,
    descriptor_pool: vk::DescriptorPool,

    worlds: HashMap<WorldId, Scene>,
    proxies: HashMap<Entity, SceneProxy>,

    // TODO: these need to be removed
    color: Image,
    depth: Image,
    // render graph should fix this
    graphics_pipeline: GraphicsPipeline,
}

impl SceneManager {
    fn init(device: &RenderDevice, size: vk::Extent2D) -> Result<Self> {
        // MAX_FRAMES_IN_FLIGHT never gets anywhere near u32::MAX
        #[allow(clippy::cast_possible_truncation)]
        let pool_sizes = [
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                // scene UBOs + object UBOs, all × frames in flight
                descriptor_count: (1 + MAX_RENDERABLES) * MAX_FRAMES_IN_FLIGHT as u32,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                // rough upper bound on material textures
                descriptor_count: MAX_RENDERABLES * MAX_FRAMES_IN_FLIGHT as u32,
            },
        ];

        // MAX_FRAMES_IN_FLIGHT never gets anywhere near u32::MAX
        #[allow(clippy::cast_possible_truncation)]
        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .pool_sizes(&pool_sizes)
            .max_sets((1 + MAX_RENDERABLES * 2) * MAX_FRAMES_IN_FLIGHT as u32);

        let descriptor_pool = unsafe { device.device.create_descriptor_pool(&pool_info, None)? };

        // TEMP
        let color_info = ImageCreateInfo {
            size,
            format: device.properties.surface_format.format,
            tiling: vk::ImageTiling::OPTIMAL,
            usage: vk::ImageUsageFlags::TRANSIENT_ATTACHMENT
                | vk::ImageUsageFlags::COLOR_ATTACHMENT,
            location: MemoryLocation::GpuOnly,
            mip_levels: 1,
            num_samples: device.properties.msaa_samples,
            aspect_flags: vk::ImageAspectFlags::COLOR,
        };
        let color = Image::create_image(device, &color_info)?;

        let depth_info = ImageCreateInfo {
            size,
            format: device.properties.depth_format,
            tiling: vk::ImageTiling::OPTIMAL,
            usage: vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            location: MemoryLocation::GpuOnly,
            mip_levels: 1,
            num_samples: device.properties.msaa_samples,
            aspect_flags: vk::ImageAspectFlags::DEPTH,
        };
        let depth = Image::create_image(device, &depth_info)?;
        let graphics_pipeline =
            GraphicsPipeline::build(device, &device.layouts, &device.properties)?;

        Ok(Self {
            device: device.clone(),
            descriptor_pool,
            worlds: HashMap::new(),
            proxies: HashMap::new(),
            color,
            depth,
            graphics_pipeline,
        })
    }
}
