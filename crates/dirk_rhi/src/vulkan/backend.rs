use std::{collections::HashMap, ffi::CString, sync::Arc};

use ash::vk;
use gpu_allocator::vulkan::{AllocationCreateDesc, AllocationScheme};

use super::{
    VulkanBackend, VulkanBindGroup, VulkanBindGroupLayout, VulkanBuffer, VulkanCommandBuffer,
    VulkanCommandPool, VulkanFence, VulkanImage, VulkanImageInner, VulkanImageView,
    VulkanImageViewInner, VulkanPipeline, VulkanPipelineLayout, VulkanSampler, VulkanSemaphore,
    VulkanShaderModule, map_error, mapping, unsupported,
};
use crate::{
    Backend, BindGroupCreateInfo, BindGroupLayoutCreateInfo, BindingResource, BindingType,
    BufferBarrier, BufferCopy, BufferCreateInfo, BufferImageCopy, ColorAttachment,
    CommandBufferBeginInfo, CommandBufferLevel, CommandBufferUsage, CommandPoolCreateInfo,
    CommandPoolFlags, DepthStencilAttachment, Draw, DrawIndexed, Fence, Filter,
    GraphicsPipelineCreateInfo, ImageBarrier, ImageBlit, ImageCopy, ImageCreateInfo, ImageLayout,
    ImageViewCreateInfo, IndexFormat, LoadOperation, PipelineLayoutCreateInfo, QueueType, Rect2D,
    RenderingInfo, Result, SamplerCreateInfo, SemaphoreKind, ShaderModuleCreateInfo, ShaderSource,
    StoreOperation, SubmitInfo, VertexBufferBinding, Viewport,
};

impl crate::backend::sealed::Sealed for VulkanBackend {}

impl Backend for VulkanBackend {
    type Buffer = VulkanBuffer;
    type Image = VulkanImage;
    type ImageView = VulkanImageView;
    type Sampler = VulkanSampler;
    type ShaderModule = VulkanShaderModule;
    type BindGroupLayout = VulkanBindGroupLayout;
    type BindGroup = VulkanBindGroup;
    type PipelineLayout = VulkanPipelineLayout;
    type Pipeline = VulkanPipeline;
    type CommandPool = VulkanCommandPool;
    type CommandBuffer = VulkanCommandBuffer;
    type Fence = VulkanFence;
    type Semaphore = VulkanSemaphore;

    fn wait_idle(&self) -> Result<()> {
        unsafe { self.inner.device.device_wait_idle() }
            .map_err(|error| map_error("wait for Vulkan device", error))
    }

    fn flush(&self) {
        self.inner.flush_deletions();
    }

    fn create_buffer(&self, info: &BufferCreateInfo<'_>) -> Result<Self::Buffer> {
        let create_info = vk::BufferCreateInfo::default()
            .size(info.size)
            .usage(mapping::buffer_usage(info.usage))
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        let raw = unsafe { self.inner.device.create_buffer(&create_info, None) }
            .map_err(|error| map_error("create Vulkan buffer", error))?;
        let requirements = unsafe { self.inner.device.get_buffer_memory_requirements(raw) };
        let allocation = self
            .inner
            .allocator()
            .as_mut()
            .expect("Vulkan allocator exists while the device is alive")
            .allocate(&AllocationCreateDesc {
                name: info.label.unwrap_or("RHI buffer"),
                requirements,
                location: mapping::memory_location(info.memory),
                linear: true,
                allocation_scheme: AllocationScheme::GpuAllocatorManaged,
            })
            .map_err(|error| map_error("allocate Vulkan buffer memory", error));
        let allocation = match allocation {
            Ok(allocation) => allocation,
            Err(error) => {
                unsafe { self.inner.device.destroy_buffer(raw, None) };
                return Err(error);
            }
        };
        if let Err(error) = unsafe {
            self.inner
                .device
                .bind_buffer_memory(raw, allocation.memory(), allocation.offset())
        } {
            unsafe { self.inner.device.destroy_buffer(raw, None) };
            self.inner
                .allocator()
                .as_mut()
                .expect("Vulkan allocator exists while the device is alive")
                .free(allocation)
                .map_err(|free_error| map_error("free Vulkan buffer memory", free_error))?;
            return Err(map_error("bind Vulkan buffer memory", error));
        }
        Ok(VulkanBuffer {
            inner: Arc::clone(&self.inner),
            raw,
            allocation: std::sync::Mutex::new(Some(allocation)),
            size: info.size,
        })
    }

    fn write_buffer(&self, buffer: &Self::Buffer, offset: u64, data: &[u8]) -> Result<()> {
        let allocation = super::lock(&buffer.allocation);
        let allocation = allocation
            .as_ref()
            .expect("live Vulkan buffer has an allocation");
        let mapped = allocation.mapped_ptr().ok_or_else(|| {
            unsupported("Vulkan buffer memory is not host-writable; use an upload buffer and copy")
        })?;
        let offset = usize::try_from(offset)
            .map_err(|_| unsupported("Vulkan buffer write offset does not fit usize"))?;
        let end = offset
            .checked_add(data.len())
            .ok_or_else(|| unsupported("Vulkan buffer write range overflows usize"))?;
        if u64::try_from(end).unwrap_or(u64::MAX) > buffer.size {
            return Err(unsupported("Vulkan buffer write exceeds the allocation"));
        }
        unsafe {
            std::ptr::copy_nonoverlapping(
                data.as_ptr(),
                mapped.as_ptr().cast::<u8>().add(offset),
                data.len(),
            );
        }
        Ok(())
    }

    fn create_image(&self, info: &ImageCreateInfo<'_>) -> Result<Self::Image> {
        match info.dimension {
            crate::ImageDimension::One if info.extent.height != 1 || info.extent.depth != 1 => {
                return Err(unsupported(
                    "one-dimensional Vulkan images require height and depth of one",
                ));
            }
            crate::ImageDimension::Two if info.extent.depth != 1 => {
                return Err(unsupported(
                    "two-dimensional Vulkan images require depth of one",
                ));
            }
            _ => {}
        }
        let create_info = vk::ImageCreateInfo::default()
            .flags(image_create_flags(info))
            .image_type(mapping::image_type(info.dimension))
            .format(mapping::format(info.format))
            .extent(vk::Extent3D {
                width: info.extent.width,
                height: info.extent.height,
                depth: info.extent.depth,
            })
            .mip_levels(info.mip_levels)
            .array_layers(info.array_layers)
            .samples(mapping::sample_count(info.samples))
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(mapping::image_usage(info.usage))
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);
        let raw = unsafe { self.inner.device.create_image(&create_info, None) }
            .map_err(|error| map_error("create Vulkan image", error))?;
        let requirements = unsafe { self.inner.device.get_image_memory_requirements(raw) };
        let allocation = self
            .inner
            .allocator()
            .as_mut()
            .expect("Vulkan allocator exists while the device is alive")
            .allocate(&AllocationCreateDesc {
                name: info.label.unwrap_or("RHI image"),
                requirements,
                location: mapping::memory_location(info.memory),
                linear: false,
                allocation_scheme: AllocationScheme::GpuAllocatorManaged,
            })
            .map_err(|error| map_error("allocate Vulkan image memory", error));
        let allocation = match allocation {
            Ok(allocation) => allocation,
            Err(error) => {
                unsafe { self.inner.device.destroy_image(raw, None) };
                return Err(error);
            }
        };
        if let Err(error) = unsafe {
            self.inner
                .device
                .bind_image_memory(raw, allocation.memory(), allocation.offset())
        } {
            unsafe { self.inner.device.destroy_image(raw, None) };
            self.inner
                .allocator()
                .as_mut()
                .expect("Vulkan allocator exists while the device is alive")
                .free(allocation)
                .map_err(|free_error| map_error("free Vulkan image memory", free_error))?;
            return Err(map_error("bind Vulkan image memory", error));
        }
        Ok(VulkanImage(Arc::new(VulkanImageInner {
            device: Arc::clone(&self.inner),
            raw,
            format: mapping::format(info.format),
            allocation: Some(allocation),
        })))
    }

    fn create_image_view(
        &self,
        image: &Self::Image,
        info: &ImageViewCreateInfo<'_>,
    ) -> Result<Self::ImageView> {
        match info.dimension {
            crate::ImageViewDimension::Cube if info.range.array_layer_count != 6 => {
                return Err(unsupported("Vulkan cube image views require six layers"));
            }
            crate::ImageViewDimension::CubeArray
                if info.range.array_layer_count < 6
                    || !info.range.array_layer_count.is_multiple_of(6) =>
            {
                return Err(unsupported(
                    "Vulkan cube-array views require a layer count divisible by six",
                ));
            }
            _ => {}
        }
        let create_info = vk::ImageViewCreateInfo::default()
            .image(image.raw())
            .view_type(mapping::image_view_type(info.dimension))
            .format(info.format.map_or_else(|| image.format(), mapping::format))
            .subresource_range(mapping::subresource_range(info.range));
        let format = create_info.format;
        let raw = unsafe { self.inner.device.create_image_view(&create_info, None) }
            .map_err(|error| map_error("create Vulkan image view", error))?;
        Ok(VulkanImageView(Arc::new(VulkanImageViewInner {
            device: Arc::clone(&self.inner),
            raw,
            format,
        })))
    }

    fn create_sampler(&self, info: &SamplerCreateInfo<'_>) -> Result<Self::Sampler> {
        let create_info = vk::SamplerCreateInfo::default()
            .mag_filter(mapping::filter(info.mag_filter))
            .min_filter(mapping::filter(info.min_filter))
            .mipmap_mode(mapping::mipmap_mode(info.mipmap_filter))
            .address_mode_u(mapping::address_mode(info.address_u))
            .address_mode_v(mapping::address_mode(info.address_v))
            .address_mode_w(mapping::address_mode(info.address_w))
            .min_lod(info.lod_min)
            .max_lod(info.lod_max)
            .anisotropy_enable(info.anisotropy)
            .max_anisotropy(if info.anisotropy {
                self.inner.max_sampler_anisotropy
            } else {
                1.0
            });
        let raw = unsafe { self.inner.device.create_sampler(&create_info, None) }
            .map_err(|error| map_error("create Vulkan sampler", error))?;
        Ok(VulkanSampler {
            inner: Arc::clone(&self.inner),
            raw,
        })
    }

    fn create_shader_module(
        &self,
        info: &ShaderModuleCreateInfo<'_>,
    ) -> Result<Self::ShaderModule> {
        let ShaderSource::SpirV(words) = info.source;
        let create_info = vk::ShaderModuleCreateInfo::default().code(words);
        let raw = unsafe { self.inner.device.create_shader_module(&create_info, None) }
            .map_err(|error| map_error("create Vulkan shader module", error))?;
        Ok(VulkanShaderModule {
            inner: Arc::clone(&self.inner),
            raw,
        })
    }

    fn create_bind_group_layout(
        &self,
        info: &BindGroupLayoutCreateInfo<'_>,
    ) -> Result<Self::BindGroupLayout> {
        let bindings = info
            .entries
            .iter()
            .map(|entry| {
                vk::DescriptorSetLayoutBinding::default()
                    .binding(entry.binding)
                    .descriptor_type(mapping::descriptor_type(entry.binding_type))
                    .descriptor_count(entry.count)
                    .stage_flags(mapping::shader_stages(entry.visibility))
            })
            .collect::<Vec<_>>();
        let create_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);
        let raw = unsafe {
            self.inner
                .device
                .create_descriptor_set_layout(&create_info, None)
        }
        .map_err(|error| map_error("create Vulkan descriptor-set layout", error))?;
        Ok(VulkanBindGroupLayout {
            inner: Arc::clone(&self.inner),
            raw,
            entries: info.entries.to_vec(),
        })
    }

    fn create_bind_group(&self, info: &BindGroupCreateInfo<'_, Self>) -> Result<Self::BindGroup> {
        let layout = info.layout.raw.as_ref();
        validate_bind_group_entries(layout, info)?;
        let mut counts = HashMap::<vk::DescriptorType, u32>::new();
        for entry in &layout.entries {
            *counts
                .entry(mapping::descriptor_type(entry.binding_type))
                .or_default() += entry.count;
        }
        let pool_sizes = counts
            .into_iter()
            .map(|(ty, descriptor_count)| vk::DescriptorPoolSize {
                ty,
                descriptor_count,
            })
            .collect::<Vec<_>>();
        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(1)
            .pool_sizes(&pool_sizes);
        let pool = unsafe { self.inner.device.create_descriptor_pool(&pool_info, None) }
            .map_err(|error| map_error("create Vulkan descriptor pool", error))?;
        let layouts = [layout.raw];
        let allocate_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(pool)
            .set_layouts(&layouts);
        let set = match unsafe { self.inner.device.allocate_descriptor_sets(&allocate_info) } {
            Ok(sets) => sets[0],
            Err(error) => {
                unsafe { self.inner.device.destroy_descriptor_pool(pool, None) };
                return Err(map_error("allocate Vulkan descriptor set", error));
            }
        };
        for entry in info.entries {
            write_descriptor(&self.inner.device, set, entry, layout)?;
        }
        Ok(VulkanBindGroup {
            inner: Arc::clone(&self.inner),
            pool,
            set,
        })
    }

    fn create_pipeline_layout(
        &self,
        info: &PipelineLayoutCreateInfo<'_, Self>,
    ) -> Result<Self::PipelineLayout> {
        let layouts = info
            .bind_group_layouts
            .iter()
            .map(|layout| layout.raw.raw)
            .collect::<Vec<_>>();
        let create_info = vk::PipelineLayoutCreateInfo::default().set_layouts(&layouts);
        let raw = unsafe { self.inner.device.create_pipeline_layout(&create_info, None) }
            .map_err(|error| map_error("create Vulkan pipeline layout", error))?;
        Ok(VulkanPipelineLayout {
            inner: Arc::clone(&self.inner),
            raw,
        })
    }

    fn create_graphics_pipeline(
        &self,
        info: &GraphicsPipelineCreateInfo<'_, Self>,
    ) -> Result<Self::Pipeline> {
        create_graphics_pipeline(self, info)
    }

    fn create_command_pool(&self, info: &CommandPoolCreateInfo<'_>) -> Result<Self::CommandPool> {
        let mut flags = vk::CommandPoolCreateFlags::empty();
        if info.flags.contains(CommandPoolFlags::TRANSIENT) {
            flags |= vk::CommandPoolCreateFlags::TRANSIENT;
        }
        if info.flags.contains(CommandPoolFlags::RESET_COMMAND_BUFFER) {
            flags |= vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER;
        }
        let create_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(self.inner.queues.get(info.queue).family_index)
            .flags(flags);
        let raw = unsafe { self.inner.device.create_command_pool(&create_info, None) }
            .map_err(|error| map_error("create Vulkan command pool", error))?;
        Ok(VulkanCommandPool {
            inner: Arc::clone(&self.inner),
            raw,
        })
    }

    fn reset_command_pool(&self, pool: &Self::CommandPool) -> Result<()> {
        unsafe {
            self.inner
                .device
                .reset_command_pool(pool.raw, vk::CommandPoolResetFlags::RELEASE_RESOURCES)
        }
        .map_err(|error| map_error("reset Vulkan command pool", error))
    }

    fn allocate_command_buffer(
        &self,
        pool: &Self::CommandPool,
        level: CommandBufferLevel,
    ) -> Result<Self::CommandBuffer> {
        let level = match level {
            CommandBufferLevel::Primary => vk::CommandBufferLevel::PRIMARY,
            CommandBufferLevel::Secondary => {
                return Err(unsupported(
                    "Vulkan secondary command buffers require inheritance metadata not represented by the RHI",
                ));
            }
        };
        let allocate_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(pool.raw)
            .level(level)
            .command_buffer_count(1);
        let raw = unsafe { self.inner.device.allocate_command_buffers(&allocate_info) }
            .map_err(|error| map_error("allocate Vulkan command buffer", error))?[0];
        Ok(VulkanCommandBuffer { raw })
    }

    fn begin_command_buffer(
        &self,
        command_buffer: &mut Self::CommandBuffer,
        info: &CommandBufferBeginInfo,
    ) -> Result<()> {
        let mut flags = vk::CommandBufferUsageFlags::empty();
        if info.usage.contains(CommandBufferUsage::ONE_TIME_SUBMIT) {
            flags |= vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT;
        }
        if info.usage.contains(CommandBufferUsage::SIMULTANEOUS_USE) {
            flags |= vk::CommandBufferUsageFlags::SIMULTANEOUS_USE;
        }
        let begin_info = vk::CommandBufferBeginInfo::default().flags(flags);
        unsafe {
            self.inner
                .device
                .begin_command_buffer(command_buffer.raw, &begin_info)
        }
        .map_err(|error| map_error("begin Vulkan command buffer", error))
    }

    fn end_command_buffer(&self, command_buffer: &mut Self::CommandBuffer) -> Result<()> {
        unsafe { self.inner.device.end_command_buffer(command_buffer.raw) }
            .map_err(|error| map_error("end Vulkan command buffer", error))
    }

    fn reset_command_buffer(&self, command_buffer: &mut Self::CommandBuffer) -> Result<()> {
        unsafe {
            self.inner.device.reset_command_buffer(
                command_buffer.raw,
                vk::CommandBufferResetFlags::RELEASE_RESOURCES,
            )
        }
        .map_err(|error| map_error("reset Vulkan command buffer", error))
    }

    fn command_barriers(
        &self,
        command_buffer: &mut Self::CommandBuffer,
        image_barriers: &[ImageBarrier<'_, Self>],
        buffer_barriers: &[BufferBarrier<'_, Self>],
    ) -> Result<()> {
        let image_barriers = image_barriers
            .iter()
            .map(|barrier| {
                vk::ImageMemoryBarrier2::default()
                    .src_stage_mask(mapping::pipeline_stages(barrier.before.stages))
                    .src_access_mask(mapping::access_types(barrier.before.access))
                    .dst_stage_mask(mapping::pipeline_stages(barrier.after.stages))
                    .dst_access_mask(mapping::access_types(barrier.after.access))
                    .old_layout(mapping::image_layout(barrier.before.layout))
                    .new_layout(mapping::image_layout(barrier.after.layout))
                    .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .image(barrier.image.raw.raw())
                    .subresource_range(mapping::subresource_range(barrier.range))
            })
            .collect::<Vec<_>>();
        let buffer_barriers = buffer_barriers
            .iter()
            .map(|barrier| {
                vk::BufferMemoryBarrier2::default()
                    .src_stage_mask(mapping::pipeline_stages(barrier.source_stages))
                    .src_access_mask(mapping::access_types(barrier.source_access))
                    .dst_stage_mask(mapping::pipeline_stages(barrier.destination_stages))
                    .dst_access_mask(mapping::access_types(barrier.destination_access))
                    .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .buffer(barrier.buffer.raw.raw)
                    .offset(barrier.offset)
                    .size(barrier.size)
            })
            .collect::<Vec<_>>();
        let dependency_info = vk::DependencyInfo::default()
            .image_memory_barriers(&image_barriers)
            .buffer_memory_barriers(&buffer_barriers);
        unsafe {
            self.inner
                .device
                .cmd_pipeline_barrier2(command_buffer.raw, &dependency_info);
        }
        Ok(())
    }

    fn command_begin_rendering(
        &self,
        command_buffer: &mut Self::CommandBuffer,
        info: &RenderingInfo<'_, Self>,
    ) -> Result<()> {
        let colors = info
            .color_attachments
            .iter()
            .map(color_attachment)
            .collect::<Vec<_>>();
        let depth = info.depth_stencil_attachment.as_ref().map(depth_attachment);
        let mut rendering_info = vk::RenderingInfo::default()
            .render_area(vk::Rect2D {
                offset: vk::Offset2D {
                    x: info.render_area.x,
                    y: info.render_area.y,
                },
                extent: vk::Extent2D {
                    width: info.render_area.width,
                    height: info.render_area.height,
                },
            })
            .layer_count(info.layer_count)
            .color_attachments(&colors);
        if let Some(depth) = &depth {
            rendering_info = rendering_info.depth_attachment(depth);
            let view = info
                .depth_stencil_attachment
                .as_ref()
                .expect("depth attachment exists when native attachment exists")
                .view
                .raw;
            if matches!(
                view.0.format,
                vk::Format::D24_UNORM_S8_UINT | vk::Format::D32_SFLOAT_S8_UINT
            ) {
                rendering_info = rendering_info.stencil_attachment(depth);
            }
        }
        unsafe {
            self.inner
                .device
                .cmd_begin_rendering(command_buffer.raw, &rendering_info);
        }
        Ok(())
    }

    fn command_end_rendering(&self, command_buffer: &mut Self::CommandBuffer) -> Result<()> {
        unsafe { self.inner.device.cmd_end_rendering(command_buffer.raw) };
        Ok(())
    }

    fn command_copy_buffer(
        &self,
        command_buffer: &mut Self::CommandBuffer,
        source: &Self::Buffer,
        destination: &Self::Buffer,
        regions: &[BufferCopy],
    ) -> Result<()> {
        let regions = regions
            .iter()
            .map(|region| vk::BufferCopy {
                src_offset: region.source_offset,
                dst_offset: region.destination_offset,
                size: region.size,
            })
            .collect::<Vec<_>>();
        unsafe {
            self.inner.device.cmd_copy_buffer(
                command_buffer.raw,
                source.raw,
                destination.raw,
                &regions,
            );
        }
        Ok(())
    }

    fn command_copy_buffer_to_image(
        &self,
        command_buffer: &mut Self::CommandBuffer,
        source: &Self::Buffer,
        destination: &Self::Image,
        layout: ImageLayout,
        regions: &[BufferImageCopy],
    ) -> Result<()> {
        let regions = regions.iter().map(buffer_image_copy).collect::<Vec<_>>();
        unsafe {
            self.inner.device.cmd_copy_buffer_to_image(
                command_buffer.raw,
                source.raw,
                destination.raw(),
                mapping::image_layout(layout),
                &regions,
            );
        }
        Ok(())
    }

    fn command_copy_image(
        &self,
        command_buffer: &mut Self::CommandBuffer,
        source: &Self::Image,
        source_layout: ImageLayout,
        destination: &Self::Image,
        destination_layout: ImageLayout,
        regions: &[ImageCopy],
    ) -> Result<()> {
        let regions = regions.iter().map(image_copy).collect::<Vec<_>>();
        unsafe {
            self.inner.device.cmd_copy_image(
                command_buffer.raw,
                source.raw(),
                mapping::image_layout(source_layout),
                destination.raw(),
                mapping::image_layout(destination_layout),
                &regions,
            );
        }
        Ok(())
    }

    fn command_blit_image(
        &self,
        command_buffer: &mut Self::CommandBuffer,
        source: &Self::Image,
        source_layout: ImageLayout,
        destination: &Self::Image,
        destination_layout: ImageLayout,
        regions: &[ImageBlit],
        filter: Filter,
    ) -> Result<()> {
        let regions = regions.iter().map(image_blit).collect::<Vec<_>>();
        unsafe {
            self.inner.device.cmd_blit_image(
                command_buffer.raw,
                source.raw(),
                mapping::image_layout(source_layout),
                destination.raw(),
                mapping::image_layout(destination_layout),
                &regions,
                mapping::filter(filter),
            );
        }
        Ok(())
    }

    fn command_set_viewport(
        &self,
        command_buffer: &mut Self::CommandBuffer,
        viewport: Viewport,
    ) -> Result<()> {
        let viewport = vk::Viewport {
            x: viewport.x,
            y: viewport.y,
            width: viewport.width,
            height: viewport.height,
            min_depth: viewport.min_depth,
            max_depth: viewport.max_depth,
        };
        unsafe {
            self.inner.device.cmd_set_viewport(
                command_buffer.raw,
                0,
                std::slice::from_ref(&viewport),
            );
        }
        Ok(())
    }

    fn command_set_scissor(
        &self,
        command_buffer: &mut Self::CommandBuffer,
        scissor: Rect2D,
    ) -> Result<()> {
        let scissor = vk::Rect2D {
            offset: vk::Offset2D {
                x: scissor.x,
                y: scissor.y,
            },
            extent: vk::Extent2D {
                width: scissor.width,
                height: scissor.height,
            },
        };
        unsafe {
            self.inner.device.cmd_set_scissor(
                command_buffer.raw,
                0,
                std::slice::from_ref(&scissor),
            );
        }
        Ok(())
    }

    fn command_bind_graphics_pipeline(
        &self,
        command_buffer: &mut Self::CommandBuffer,
        pipeline: &Self::Pipeline,
    ) -> Result<()> {
        unsafe {
            self.inner.device.cmd_bind_pipeline(
                command_buffer.raw,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.raw,
            );
        }
        Ok(())
    }

    fn command_bind_graphics_groups(
        &self,
        command_buffer: &mut Self::CommandBuffer,
        layout: &Self::PipelineLayout,
        first_group: u32,
        groups: &[&crate::BindGroup<Self>],
        dynamic_offsets: &[u32],
    ) -> Result<()> {
        if !dynamic_offsets.is_empty() {
            return Err(unsupported(
                "Vulkan dynamic descriptor offsets require a dynamic binding type not represented by the RHI",
            ));
        }
        let sets = groups.iter().map(|group| group.raw.set).collect::<Vec<_>>();
        unsafe {
            self.inner.device.cmd_bind_descriptor_sets(
                command_buffer.raw,
                vk::PipelineBindPoint::GRAPHICS,
                layout.raw,
                first_group,
                &sets,
                dynamic_offsets,
            );
        }
        Ok(())
    }

    fn command_bind_vertex_buffers(
        &self,
        command_buffer: &mut Self::CommandBuffer,
        first_binding: u32,
        buffers: &[VertexBufferBinding<'_, Self>],
    ) -> Result<()> {
        let handles = buffers
            .iter()
            .map(|binding| binding.buffer.raw.raw)
            .collect::<Vec<_>>();
        let offsets = buffers
            .iter()
            .map(|binding| binding.offset)
            .collect::<Vec<_>>();
        unsafe {
            self.inner.device.cmd_bind_vertex_buffers(
                command_buffer.raw,
                first_binding,
                &handles,
                &offsets,
            );
        }
        Ok(())
    }

    fn command_bind_index_buffer(
        &self,
        command_buffer: &mut Self::CommandBuffer,
        buffer: &Self::Buffer,
        offset: u64,
        format: IndexFormat,
    ) -> Result<()> {
        unsafe {
            self.inner.device.cmd_bind_index_buffer(
                command_buffer.raw,
                buffer.raw,
                offset,
                mapping::index_type(format),
            );
        }
        Ok(())
    }

    fn command_draw_indexed(
        &self,
        command_buffer: &mut Self::CommandBuffer,
        draw: DrawIndexed,
    ) -> Result<()> {
        unsafe {
            self.inner.device.cmd_draw_indexed(
                command_buffer.raw,
                draw.index_count,
                draw.instance_count,
                draw.first_index,
                draw.vertex_offset,
                draw.first_instance,
            );
        }
        Ok(())
    }

    fn command_draw(&self, command_buffer: &mut Self::CommandBuffer, draw: Draw) -> Result<()> {
        unsafe {
            self.inner.device.cmd_draw(
                command_buffer.raw,
                draw.vertex_count,
                draw.instance_count,
                draw.first_vertex,
                draw.first_instance,
            );
        }
        Ok(())
    }

    fn create_fence(&self, signaled: bool) -> Result<Self::Fence> {
        let flags = if signaled {
            vk::FenceCreateFlags::SIGNALED
        } else {
            vk::FenceCreateFlags::empty()
        };
        let create_info = vk::FenceCreateInfo::default().flags(flags);
        let raw = unsafe { self.inner.device.create_fence(&create_info, None) }
            .map_err(|error| map_error("create Vulkan fence", error))?;
        Ok(VulkanFence {
            inner: Arc::clone(&self.inner),
            raw,
        })
    }

    fn wait_for_fence(&self, fence: &Self::Fence, timeout_ns: u64) -> Result<()> {
        unsafe {
            self.inner
                .device
                .wait_for_fences(std::slice::from_ref(&fence.raw), true, timeout_ns)
        }
        .map_err(|error| map_error("wait for Vulkan fence", error))
    }

    fn reset_fence(&self, fence: &mut Self::Fence) -> Result<()> {
        unsafe {
            self.inner
                .device
                .reset_fences(std::slice::from_ref(&fence.raw))
        }
        .map_err(|error| map_error("reset Vulkan fence", error))
    }

    fn create_semaphore(&self, kind: SemaphoreKind) -> Result<Self::Semaphore> {
        let raw = match kind {
            SemaphoreKind::Binary => unsafe {
                self.inner
                    .device
                    .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
            },
            SemaphoreKind::Timeline { initial_value } => {
                let mut type_info = vk::SemaphoreTypeCreateInfo::default()
                    .semaphore_type(vk::SemaphoreType::TIMELINE)
                    .initial_value(initial_value);
                let create_info = vk::SemaphoreCreateInfo::default().push_next(&mut type_info);
                unsafe { self.inner.device.create_semaphore(&create_info, None) }
            }
        }
        .map_err(|error| map_error("create Vulkan semaphore", error))?;
        Ok(VulkanSemaphore {
            inner: Arc::clone(&self.inner),
            raw,
        })
    }

    fn wait_for_semaphore(
        &self,
        semaphore: &Self::Semaphore,
        value: u64,
        timeout_ns: u64,
    ) -> Result<()> {
        let semaphores = [semaphore.raw];
        let values = [value];
        let wait_info = vk::SemaphoreWaitInfo::default()
            .semaphores(&semaphores)
            .values(&values);
        unsafe { self.inner.device.wait_semaphores(&wait_info, timeout_ns) }
            .map_err(|error| map_error("wait for Vulkan timeline semaphore", error))
    }

    fn semaphore_value(&self, semaphore: &Self::Semaphore) -> Result<u64> {
        unsafe { self.inner.device.get_semaphore_counter_value(semaphore.raw) }
            .map_err(|error| map_error("query Vulkan timeline semaphore", error))
    }

    fn submit(
        &self,
        queue: QueueType,
        info: &SubmitInfo<'_, Self>,
        fence: &Fence<Self>,
    ) -> Result<()> {
        let waits = info
            .waits
            .iter()
            .map(|wait| {
                vk::SemaphoreSubmitInfo::default()
                    .semaphore(wait.semaphore.raw.raw)
                    .value(wait.value.unwrap_or(0))
                    .stage_mask(mapping::pipeline_stages(wait.stages))
            })
            .collect::<Vec<_>>();
        let command_buffers = info
            .command_buffers
            .iter()
            .map(|command_buffer| {
                vk::CommandBufferSubmitInfo::default().command_buffer(command_buffer.raw.raw)
            })
            .collect::<Vec<_>>();
        let signals = info
            .signals
            .iter()
            .map(|signal| {
                vk::SemaphoreSubmitInfo::default()
                    .semaphore(signal.semaphore.raw.raw)
                    .value(signal.value.unwrap_or(0))
                    .stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
            })
            .collect::<Vec<_>>();
        let submit_info = vk::SubmitInfo2::default()
            .wait_semaphore_infos(&waits)
            .command_buffer_infos(&command_buffers)
            .signal_semaphore_infos(&signals);
        let _queue_guard = super::lock(&self.inner.queue_lock);
        unsafe {
            self.inner.device.queue_submit2(
                self.inner.queues.get(queue).raw,
                std::slice::from_ref(&submit_info),
                fence.raw.raw,
            )
        }
        .map_err(|error| map_error("submit Vulkan queue", error))
    }

    #[cfg(feature = "presentation")]
    type SurfaceTarget = dyn super::VulkanSurfaceTarget;
    #[cfg(feature = "presentation")]
    type Surface = super::presentation::VulkanSurface;
    #[cfg(feature = "presentation")]
    type Swapchain = super::presentation::VulkanSwapchain;
    #[cfg(feature = "presentation")]
    type RenderImage = super::presentation::VulkanRenderImage;

    #[cfg(feature = "presentation")]
    fn create_surface(&self, target: &Self::SurfaceTarget) -> Result<Self::Surface> {
        super::presentation::create_surface(self, target)
    }

    #[cfg(feature = "presentation")]
    fn create_swapchain(
        &self,
        surface: &Self::Surface,
        info: &crate::SwapchainCreateInfo<'_>,
    ) -> Result<Self::Swapchain> {
        super::presentation::create_swapchain(self, surface, info)
    }

    #[cfg(feature = "presentation")]
    fn recreate_swapchain(
        &self,
        swapchain: &mut Self::Swapchain,
        surface: &Self::Surface,
        info: &crate::SwapchainCreateInfo<'_>,
    ) -> Result<()> {
        super::presentation::recreate_swapchain(self, swapchain, surface, info)
    }

    #[cfg(feature = "presentation")]
    fn swapchain_extent(swapchain: &Self::Swapchain) -> crate::Extent2D {
        super::presentation::swapchain_extent(swapchain)
    }

    #[cfg(feature = "presentation")]
    fn swapchain_format(swapchain: &Self::Swapchain) -> crate::Format {
        super::presentation::swapchain_format(swapchain)
    }

    #[cfg(feature = "presentation")]
    fn acquire_render_image(
        &self,
        swapchain: &mut Self::Swapchain,
        timeout_ns: u64,
        signal: &Self::Semaphore,
    ) -> Result<Self::RenderImage> {
        super::presentation::acquire_render_image(self, swapchain, timeout_ns, signal)
    }

    #[cfg(feature = "presentation")]
    fn render_image_parts(image: &Self::RenderImage) -> (&Self::Image, &Self::ImageView, u32) {
        super::presentation::render_image_parts(image)
    }

    #[cfg(feature = "presentation")]
    fn present(
        &self,
        swapchain: &mut Self::Swapchain,
        image: Self::RenderImage,
        waits: &[&Self::Semaphore],
    ) -> Result<()> {
        super::presentation::present(self, swapchain, &image, waits)
    }

    #[cfg(feature = "presentation")]
    fn abandon_render_image(
        &self,
        swapchain: &mut Self::Swapchain,
        image: Self::RenderImage,
    ) -> Result<()> {
        super::presentation::abandon_render_image(self, swapchain, &image)
    }
}

fn validate_bind_group_entries(
    layout: &VulkanBindGroupLayout,
    info: &BindGroupCreateInfo<'_, VulkanBackend>,
) -> Result<()> {
    for entry in info.entries {
        let layout_entry = layout
            .entries
            .iter()
            .find(|candidate| candidate.binding == entry.binding)
            .ok_or_else(|| {
                unsupported(format!(
                    "Vulkan bind group binding {} is not declared by its layout",
                    entry.binding
                ))
            })?;
        if entry.array_element >= layout_entry.count {
            return Err(unsupported(format!(
                "Vulkan bind group binding {} array element {} exceeds count {}",
                entry.binding, entry.array_element, layout_entry.count
            )));
        }
        let matches = matches!(
            (layout_entry.binding_type, entry.resource),
            (
                BindingType::UniformBuffer | BindingType::StorageBuffer { .. },
                BindingResource::Buffer { .. }
            ) | (
                BindingType::SampledImage | BindingType::StorageImage { .. },
                BindingResource::ImageView(_)
            ) | (BindingType::Sampler, BindingResource::Sampler(_))
                | (
                    BindingType::CombinedImageSampler,
                    BindingResource::CombinedImageSampler { .. }
                )
        );
        if !matches {
            return Err(unsupported(format!(
                "Vulkan bind group binding {} has a resource incompatible with {:?}",
                entry.binding, layout_entry.binding_type
            )));
        }
    }
    Ok(())
}

fn write_descriptor(
    device: &ash::Device,
    set: vk::DescriptorSet,
    entry: &crate::BindGroupEntry<'_, VulkanBackend>,
    layout: &VulkanBindGroupLayout,
) -> Result<()> {
    let layout_entry = layout
        .entries
        .iter()
        .find(|candidate| candidate.binding == entry.binding)
        .expect("bind group entries were validated");
    let descriptor_type = mapping::descriptor_type(layout_entry.binding_type);
    match entry.resource {
        BindingResource::Buffer {
            buffer,
            offset,
            size,
        } => {
            let info = [vk::DescriptorBufferInfo {
                buffer: buffer.raw.raw,
                offset,
                range: size,
            }];
            let write = vk::WriteDescriptorSet::default()
                .dst_set(set)
                .dst_binding(entry.binding)
                .dst_array_element(entry.array_element)
                .descriptor_type(descriptor_type)
                .buffer_info(&info);
            unsafe { device.update_descriptor_sets(std::slice::from_ref(&write), &[]) };
        }
        BindingResource::ImageView(view) => {
            let image_layout = match layout_entry.binding_type {
                BindingType::SampledImage => vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                BindingType::StorageImage { .. } => vk::ImageLayout::GENERAL,
                _ => {
                    return Err(unsupported(
                        "Vulkan image descriptor type was not validated",
                    ));
                }
            };
            let info = [vk::DescriptorImageInfo::default()
                .image_view(view.as_ref().raw.raw())
                .image_layout(image_layout)];
            let write = vk::WriteDescriptorSet::default()
                .dst_set(set)
                .dst_binding(entry.binding)
                .dst_array_element(entry.array_element)
                .descriptor_type(descriptor_type)
                .image_info(&info);
            unsafe { device.update_descriptor_sets(std::slice::from_ref(&write), &[]) };
        }
        BindingResource::Sampler(sampler) => {
            let info = [vk::DescriptorImageInfo::default().sampler(sampler.raw.raw)];
            let write = vk::WriteDescriptorSet::default()
                .dst_set(set)
                .dst_binding(entry.binding)
                .dst_array_element(entry.array_element)
                .descriptor_type(descriptor_type)
                .image_info(&info);
            unsafe { device.update_descriptor_sets(std::slice::from_ref(&write), &[]) };
        }
        BindingResource::CombinedImageSampler { view, sampler } => {
            let info = [vk::DescriptorImageInfo::default()
                .sampler(sampler.raw.raw)
                .image_view(view.as_ref().raw.raw())
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];
            let write = vk::WriteDescriptorSet::default()
                .dst_set(set)
                .dst_binding(entry.binding)
                .dst_array_element(entry.array_element)
                .descriptor_type(descriptor_type)
                .image_info(&info);
            unsafe { device.update_descriptor_sets(std::slice::from_ref(&write), &[]) };
        }
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn create_graphics_pipeline(
    backend: &VulkanBackend,
    info: &GraphicsPipelineCreateInfo<'_, VulkanBackend>,
) -> Result<VulkanPipeline> {
    let vertex_name = CString::new(info.vertex.entry_point)
        .map_err(|_| unsupported("Vulkan vertex entry point contains an interior NUL"))?;
    let fragment_name = info
        .fragment
        .as_ref()
        .map(|fragment| CString::new(fragment.entry_point))
        .transpose()
        .map_err(|_| unsupported("Vulkan fragment entry point contains an interior NUL"))?;
    let mut stages = vec![
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(info.vertex.module.raw.raw)
            .name(&vertex_name),
    ];
    if let (Some(fragment), Some(name)) = (&info.fragment, &fragment_name) {
        stages.push(
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(fragment.module.raw.raw)
                .name(name),
        );
    }

    let vertex_bindings = info
        .vertex
        .buffers
        .iter()
        .enumerate()
        .map(|(binding, layout)| {
            Ok(vk::VertexInputBindingDescription {
                binding: u32::try_from(binding)
                    .map_err(|_| unsupported("Vulkan vertex binding index exceeds u32"))?,
                stride: u32::try_from(layout.stride)
                    .map_err(|_| unsupported("Vulkan vertex stride exceeds u32"))?,
                input_rate: mapping::vertex_rate(layout.step_mode),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let mut vertex_attributes = Vec::new();
    for (binding, layout) in info.vertex.buffers.iter().enumerate() {
        let binding = u32::try_from(binding)
            .map_err(|_| unsupported("Vulkan vertex binding index exceeds u32"))?;
        for attribute in layout.attributes {
            vertex_attributes.push(vk::VertexInputAttributeDescription {
                location: attribute.location,
                binding,
                format: mapping::vertex_format(attribute.format),
                offset: u32::try_from(attribute.offset)
                    .map_err(|_| unsupported("Vulkan vertex attribute offset exceeds u32"))?,
            });
        }
    }
    let vertex_input = vk::PipelineVertexInputStateCreateInfo::default()
        .vertex_binding_descriptions(&vertex_bindings)
        .vertex_attribute_descriptions(&vertex_attributes);
    let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
        .topology(mapping::primitive_topology(info.primitive.topology));
    let viewport_state = vk::PipelineViewportStateCreateInfo::default()
        .viewport_count(1)
        .scissor_count(1);
    let rasterization = vk::PipelineRasterizationStateCreateInfo::default()
        .polygon_mode(vk::PolygonMode::FILL)
        .cull_mode(mapping::cull_mode(info.primitive.cull_mode))
        .front_face(mapping::front_face(info.primitive.front_face))
        .line_width(1.0);
    let multisample = vk::PipelineMultisampleStateCreateInfo::default()
        .rasterization_samples(mapping::sample_count(info.samples));
    let color_attachments = info
        .fragment
        .as_ref()
        .map_or(&[][..], |fragment| fragment.targets)
        .iter()
        .map(|target| {
            vk::PipelineColorBlendAttachmentState::default()
                .color_write_mask(mapping::color_writes(target.write_mask))
        })
        .collect::<Vec<_>>();
    let color_blend =
        vk::PipelineColorBlendStateCreateInfo::default().attachments(&color_attachments);
    let depth_stencil =
        info.depth
            .map_or_else(vk::PipelineDepthStencilStateCreateInfo::default, |depth| {
                vk::PipelineDepthStencilStateCreateInfo::default()
                    .depth_test_enable(depth.test_enabled)
                    .depth_write_enable(depth.write_enabled)
                    .depth_compare_op(mapping::compare_op(depth.compare))
            });
    let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dynamic = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);
    let color_formats = info
        .fragment
        .as_ref()
        .map_or(&[][..], |fragment| fragment.targets)
        .iter()
        .map(|target| mapping::format(target.format))
        .collect::<Vec<_>>();
    let (depth_format, stencil_format) =
        pipeline_depth_stencil_formats(info.depth.map(|d| d.format));
    let mut rendering = vk::PipelineRenderingCreateInfo::default()
        .color_attachment_formats(&color_formats)
        .depth_attachment_format(depth_format)
        .stencil_attachment_format(stencil_format);
    let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
        .stages(&stages)
        .vertex_input_state(&vertex_input)
        .input_assembly_state(&input_assembly)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterization)
        .multisample_state(&multisample)
        .color_blend_state(&color_blend)
        .depth_stencil_state(&depth_stencil)
        .dynamic_state(&dynamic)
        .layout(info.layout.raw.raw)
        .push_next(&mut rendering);
    let raw = unsafe {
        backend.inner.device.create_graphics_pipelines(
            vk::PipelineCache::null(),
            std::slice::from_ref(&pipeline_info),
            None,
        )
    }
    .map_err(|(_, error)| map_error("create Vulkan graphics pipeline", error))?[0];
    Ok(VulkanPipeline {
        inner: Arc::clone(&backend.inner),
        raw,
    })
}

fn image_create_flags(info: &ImageCreateInfo<'_>) -> vk::ImageCreateFlags {
    if info.dimension == crate::ImageDimension::Two
        && info.extent.width == info.extent.height
        && info.array_layers >= 6
    {
        vk::ImageCreateFlags::CUBE_COMPATIBLE
    } else {
        vk::ImageCreateFlags::empty()
    }
}

fn pipeline_depth_stencil_formats(format: Option<crate::Format>) -> (vk::Format, vk::Format) {
    let depth = format.map_or(vk::Format::UNDEFINED, mapping::format);
    let stencil = format
        .filter(|format| format.aspects().contains(crate::ImageAspects::STENCIL))
        .map_or(vk::Format::UNDEFINED, mapping::format);
    (depth, stencil)
}

fn color_attachment(
    attachment: &ColorAttachment<'_, VulkanBackend>,
) -> vk::RenderingAttachmentInfo<'static> {
    let (load_op, clear_value) = match attachment.load {
        LoadOperation::Load => (vk::AttachmentLoadOp::LOAD, vk::ClearValue::default()),
        LoadOperation::Clear(color) => (
            vk::AttachmentLoadOp::CLEAR,
            vk::ClearValue {
                color: vk::ClearColorValue { float32: color },
            },
        ),
        LoadOperation::DontCare => (vk::AttachmentLoadOp::DONT_CARE, vk::ClearValue::default()),
    };
    let mut info = vk::RenderingAttachmentInfo::default()
        .image_view(attachment.view.raw.raw())
        .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .load_op(load_op)
        .store_op(store_op(attachment.store))
        .clear_value(clear_value);
    if let Some(resolve) = attachment.resolve_target {
        info = info
            .resolve_mode(vk::ResolveModeFlags::AVERAGE)
            .resolve_image_view(resolve.raw.raw())
            .resolve_image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);
    }
    info
}

fn depth_attachment(
    attachment: &DepthStencilAttachment<'_, VulkanBackend>,
) -> vk::RenderingAttachmentInfo<'static> {
    let (load_op, clear_value) = match attachment.load {
        LoadOperation::Load => (vk::AttachmentLoadOp::LOAD, vk::ClearValue::default()),
        LoadOperation::Clear(value) => (
            vk::AttachmentLoadOp::CLEAR,
            vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: value.depth,
                    stencil: value.stencil,
                },
            },
        ),
        LoadOperation::DontCare => (vk::AttachmentLoadOp::DONT_CARE, vk::ClearValue::default()),
    };
    vk::RenderingAttachmentInfo::default()
        .image_view(attachment.view.raw.raw())
        .image_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
        .load_op(load_op)
        .store_op(store_op(attachment.store))
        .clear_value(clear_value)
}

fn store_op(value: StoreOperation) -> vk::AttachmentStoreOp {
    match value {
        StoreOperation::Store => vk::AttachmentStoreOp::STORE,
        StoreOperation::DontCare => vk::AttachmentStoreOp::DONT_CARE,
    }
}

fn buffer_image_copy(value: &BufferImageCopy) -> vk::BufferImageCopy {
    vk::BufferImageCopy::default()
        .buffer_offset(value.buffer_offset)
        .image_subresource(mapping::subresource_layers(value.image_subresource))
        .image_offset(offset(value.image_offset))
        .image_extent(extent(value.image_extent))
}

fn image_copy(value: &ImageCopy) -> vk::ImageCopy {
    vk::ImageCopy::default()
        .src_subresource(mapping::subresource_layers(value.source_subresource))
        .src_offset(offset(value.source_offset))
        .dst_subresource(mapping::subresource_layers(value.destination_subresource))
        .dst_offset(offset(value.destination_offset))
        .extent(extent(value.extent))
}

fn image_blit(value: &ImageBlit) -> vk::ImageBlit {
    vk::ImageBlit::default()
        .src_subresource(mapping::subresource_layers(value.source_subresource))
        .src_offsets(value.source_offsets.map(offset))
        .dst_subresource(mapping::subresource_layers(value.destination_subresource))
        .dst_offsets(value.destination_offsets.map(offset))
}

fn offset(value: crate::Offset3D) -> vk::Offset3D {
    vk::Offset3D {
        x: value.x,
        y: value.y,
        z: value.z,
    }
}

fn extent(value: crate::Extent3D) -> vk::Extent3D {
    vk::Extent3D {
        width: value.width,
        height: value.height,
        depth: value.depth,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image_info(width: u32, height: u32, layers: u32) -> ImageCreateInfo<'static> {
        ImageCreateInfo {
            dimension: crate::ImageDimension::Two,
            extent: crate::Extent3D::new(width, height, 1),
            format: crate::Format::Rgba8Unorm,
            usage: crate::ImageUsage::SAMPLED,
            memory: crate::MemoryLocation::Device,
            mip_levels: 1,
            array_layers: layers,
            samples: crate::SampleCount::One,
            label: None,
        }
    }

    #[test]
    fn cube_compatible_flag_requires_square_2d_array() {
        assert_eq!(
            image_create_flags(&image_info(64, 64, 6)),
            vk::ImageCreateFlags::CUBE_COMPATIBLE
        );
        assert!(image_create_flags(&image_info(64, 32, 6)).is_empty());
        assert!(image_create_flags(&image_info(64, 64, 5)).is_empty());
    }

    #[test]
    fn stencil_depth_formats_set_both_dynamic_rendering_formats() {
        assert_eq!(
            pipeline_depth_stencil_formats(Some(crate::Format::Depth24UnormStencil8)),
            (vk::Format::D24_UNORM_S8_UINT, vk::Format::D24_UNORM_S8_UINT)
        );
        assert_eq!(
            pipeline_depth_stencil_formats(Some(crate::Format::Depth32Float)),
            (vk::Format::D32_SFLOAT, vk::Format::UNDEFINED)
        );
    }
}
