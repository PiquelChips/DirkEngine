use std::{
    ops::Deref,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

#[cfg(validation)]
use ash::ext::debug_utils;
use ash::{
    khr::{surface, swapchain},
    vk,
};
use gpu_allocator::{
    AllocationSizes, AllocatorDebugSettings,
    vulkan::{Allocation, AllocationCreateDesc, Allocator, AllocatorCreateDesc},
};
use parking_lot::Mutex;

use crate::{
    DescriptorLayouts, MAX_FRAMES_IN_FLIGHT, Queues, RendererProperties, Result,
    resources::command_pool::{CommandPool, Graphics, Transfer},
};

/// The device that stores all vulkan objects.
#[derive(Clone)]
pub struct RenderDevice(Arc<RenderDeviceInner>);

impl Deref for RenderDevice {
    type Target = RenderDeviceInner;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct RenderDeviceInner {
    pub device: ash::Device,
    pub surface_loader: surface::Instance,
    pub swapchain_loader: swapchain::Device,

    pub instance: ash::Instance,
    pub physical_device: vk::PhysicalDevice,
    pub queues: Queues,
    /// For single use buffers.
    /// Used for texture uploads and layout transitions.
    pub transfer_pool: CommandPool<Transfer>,
    /// For single use buffers.
    /// Used for mip generation.
    pub graphics_pool: CommandPool<Graphics>,
    /// All the descriptor layouts used in the renderer.
    pub layouts: DescriptorLayouts,
    pub properties: RendererProperties,

    #[cfg(validation)]
    debug_utils_loader: debug_utils::Instance,
    #[cfg(validation)]
    debug_messenger: vk::DebugUtilsMessengerEXT,

    allocator: Mutex<Allocator>,
    deletion_queue: Mutex<DeletionQueue>,
    current_frame: Arc<AtomicUsize>,
}

impl RenderDevice {
    pub fn new(
        entry: &ash::Entry,
        instance: ash::Instance,
        device: ash::Device,
        physical_device: vk::PhysicalDevice,
        properties: RendererProperties,
        current_frame: Arc<AtomicUsize>,
        #[cfg(validation)] debug_messenger: vk::DebugUtilsMessengerEXT,
    ) -> Result<Self> {
        // ALLOCATOR
        let allocator = Allocator::new(&AllocatorCreateDesc {
            instance: instance.clone(),
            device: device.clone(),
            physical_device,
            debug_settings: AllocatorDebugSettings::default(),
            buffer_device_address: true,
            allocation_sizes: AllocationSizes::default(),
        })?;

        // SWAP CHAIN
        let swapchain_loader = swapchain::Device::new(&instance, &device);

        // LAYOUTS
        let layouts = DescriptorLayouts {
            scene: {
                let binding = vk::DescriptorSetLayoutBinding::default()
                    .binding(0)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::VERTEX);

                let info = vk::DescriptorSetLayoutCreateInfo::default()
                    .bindings(std::slice::from_ref(&binding));

                unsafe { device.create_descriptor_set_layout(&info, None)? }
            },
            object: {
                let binding = vk::DescriptorSetLayoutBinding::default()
                    .binding(1)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::VERTEX);

                let info = vk::DescriptorSetLayoutCreateInfo::default()
                    .bindings(std::slice::from_ref(&binding));

                unsafe { device.create_descriptor_set_layout(&info, None)? }
            },
            material: {
                let binding = vk::DescriptorSetLayoutBinding::default()
                    .binding(2)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::FRAGMENT);

                let info = vk::DescriptorSetLayoutCreateInfo::default()
                    .bindings(std::slice::from_ref(&binding));

                unsafe { device.create_descriptor_set_layout(&info, None)? }
            },
        };

        // QUEUES
        let queues = {
            let indices = &properties.queue_family_indices;
            Queues {
                graphics: unsafe { device.get_device_queue(indices.graphics, 0) },
                present: unsafe { device.get_device_queue(indices.present, 0) },
                compute: unsafe { device.get_device_queue(indices.compute, 0) },
                transfer: unsafe { device.get_device_queue(indices.transfer, 0) },
            }
        };

        // COMMAND POOLS
        let transfer_pool = CommandPool::build(
            &device,
            &queues,
            &properties.queue_family_indices,
            vk::CommandPoolCreateFlags::TRANSIENT,
        )?;
        let graphics_pool = CommandPool::build(
            &device,
            &queues,
            &properties.queue_family_indices,
            vk::CommandPoolCreateFlags::TRANSIENT,
        )?;

        Ok(Self(Arc::new(RenderDeviceInner {
            device,
            surface_loader: surface::Instance::new(entry, &instance),
            swapchain_loader,
            physical_device,
            queues,
            transfer_pool,
            graphics_pool,
            layouts,
            properties,
            allocator: Mutex::new(allocator),
            deletion_queue: Mutex::new(DeletionQueue::new(
                current_frame.clone(),
                MAX_FRAMES_IN_FLIGHT,
            )),
            current_frame,

            #[cfg(validation)]
            debug_utils_loader: debug_utils::Instance::new(entry, &instance),
            #[cfg(validation)]
            debug_messenger,

            instance,
        })))
    }

    pub fn allocate(&self, desc: &AllocationCreateDesc<'_>) -> Result<Allocation> {
        Ok(self.allocator.lock().allocate(desc)?)
    }
    pub fn free(&self, allocation: Allocation) -> Result<()> {
        Ok(self.allocator.lock().free(allocation)?)
    }

    pub fn destroy(&mut self, garbage: Garbage) {
        self.deletion_queue.lock().enqueue(garbage);
    }

    /// Call once per frame from your render loop.
    pub fn flush_deletions(&self) {
        let mut queue = self.deletion_queue.lock();
        queue.flush(self, self.current_frame());
    }

    /// Call once before shutdown to flush the entire queue
    pub fn flush_all(&self) {
        let mut queue = self.deletion_queue.lock();
        queue.flush_all(self);
    }

    pub fn current_frame(&self) -> usize {
        self.current_frame.load(Ordering::Relaxed)
    }
}

impl Drop for RenderDeviceInner {
    fn drop(&mut self) {
        self.layouts.destroy(&self.device);
        self.graphics_pool.destroy();
        self.transfer_pool.destroy();
        unsafe {
            self.device.destroy_device(None);
            #[cfg(validation)]
            self.debug_utils_loader
                .destroy_debug_utils_messenger(self.debug_messenger, None);

            self.instance.destroy_instance(None);
        }
    }
}

/// Everything that can be enqueued for deferred destruction.
/// Add variants as you add resource types.
pub enum Garbage {
    Allocation(Allocation),
    Buffer(vk::Buffer),
    Image(vk::Image),
    ImageView(vk::ImageView),
    Sampler(vk::Sampler),
    Pipeline(vk::Pipeline),
    PipelineLayout(vk::PipelineLayout),
    DescriptorPool(vk::DescriptorPool),
    Semaphore(vk::Semaphore),

    Swapchain(vk::SwapchainKHR),
    Surface(vk::SurfaceKHR),
}

struct PendingDeletion {
    garbage: Option<Garbage>,
    death_frame: usize, // frame index after which it's safe to delete
}

pub struct DeletionQueue {
    pending: Vec<PendingDeletion>,
    current_frame: Arc<AtomicUsize>,
    frames_in_flight: usize,
}

impl DeletionQueue {
    pub fn new(current_frame: Arc<AtomicUsize>, frames_in_flight: usize) -> Self {
        Self {
            pending: Vec::new(),
            current_frame,
            frames_in_flight,
        }
    }

    /// Call this when you're done with a resource.
    pub fn enqueue(&mut self, garbage: Garbage) {
        self.pending.push(PendingDeletion {
            garbage: Some(garbage),
            death_frame: self.current_frame.load(Ordering::Relaxed) + 2 * self.frames_in_flight, // wait two frames before destroying
        });
    }

    /// Call once per frame. Destroys anything safe to destroy.
    pub fn flush(&mut self, device: &RenderDevice, current_frame: usize) {
        self.pending.retain_mut(|item| {
            if current_frame >= item.death_frame {
                if let Some(garbage) = item.garbage.take() {
                    garbage.destroy(device);
                }
                false
            } else {
                true
            }
        });
    }

    /// Call on shutdown — destroys everything regardless of frame.
    pub fn flush_all(&mut self, device: &RenderDevice) {
        for mut item in self.pending.drain(..) {
            if let Some(garbage) = item.garbage.take() {
                garbage.destroy(device);
            }
        }
    }
}

impl Garbage {
    fn destroy(self, render_device: &RenderDevice) {
        let device = &render_device.device;
        unsafe {
            match self {
                Self::Allocation(allocation) => render_device
                    .free(allocation)
                    .expect("Failed to free allocation"),
                Self::Buffer(buffer) => device.destroy_buffer(buffer, None),
                Self::Image(image) => device.destroy_image(image, None),
                Self::ImageView(view) => device.destroy_image_view(view, None),
                Self::Sampler(s) => device.destroy_sampler(s, None),
                Self::Pipeline(p) => device.destroy_pipeline(p, None),
                Self::PipelineLayout(l) => device.destroy_pipeline_layout(l, None),
                Self::DescriptorPool(pool) => device.destroy_descriptor_pool(pool, None),
                Self::Semaphore(semaphore) => device.destroy_semaphore(semaphore, None),
                Self::Surface(surface) => {
                    render_device.surface_loader.destroy_surface(surface, None);
                }
                Self::Swapchain(swapchain) => render_device
                    .swapchain_loader
                    .destroy_swapchain(swapchain, None),
            }
        }
    }
}
