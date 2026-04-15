use std::{
    ops::Deref,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use ash::{
    khr::{surface, swapchain},
    vk,
};
use gpu_allocator::vulkan::{Allocation, Allocator};
use parking_lot::Mutex;

use crate::{
    DescriptorLayouts, MAX_FRAMES_IN_FLIGHT, Queues, RendererProperties,
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

    pub allocator: Mutex<Allocator>,
    pub event_manager: events::EventManager,
    deletion_queue: Mutex<DeletionQueue>,
    current_frame: Arc<AtomicU64>,
}

impl RenderDevice {
    pub fn new(
        instance: ash::Instance,
        device: ash::Device,
        surface_loader: surface::Instance,
        swapchain_loader: swapchain::Device,
        physical_device: vk::PhysicalDevice,
        queues: Queues,
        transfer_pool: CommandPool<Transfer>,
        graphics_pool: CommandPool<Graphics>,
        layouts: DescriptorLayouts,
        properties: RendererProperties,
        allocator: Allocator,
        current_frame: Arc<AtomicU64>,
        event_manager: events::EventManager,
    ) -> Self {
        Self(Arc::new(RenderDeviceInner {
            device,
            surface_loader,
            swapchain_loader,
            instance,
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
            event_manager,
        }))
    }

    pub fn destroy(&mut self, garbage: Garbage) {
        self.deletion_queue.lock().enqueue(garbage);
    }

    /// Call once per frame from your render loop.
    pub fn flush_deletions(&self) {
        let mut queue = self.deletion_queue.lock();
        let mut allocator = self.allocator.lock();
        queue.flush(&self.device, &mut allocator, self.current_frame());
    }

    /// Call during shutdown, after device_wait_idle.
    pub fn shutdown(&self) {
        unsafe {
            self.device.device_wait_idle().unwrap();
        }
        let mut queue = self.deletion_queue.lock();
        let mut allocator = self.allocator.lock();
        queue.flush_all(&self.device, &mut allocator);
    }

    pub fn current_frame(&self) -> u64 {
        self.current_frame.load(Ordering::Relaxed)
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
    DescriptorPool(vk::DescriptorPool),
    DescriptorSetLayout(vk::DescriptorSetLayout),
    Semaphore(vk::Semaphore),

    Swapchain(vk::SwapchainKHR),
    Surface(vk::SurfaceKHR),
}

struct PendingDeletion {
    garbage: Option<Garbage>,
    death_frame: u64, // frame index after which it's safe to delete
}

pub struct DeletionQueue {
    pending: Vec<PendingDeletion>,
    current_frame: Arc<AtomicU64>,
    frames_in_flight: usize,
}

impl DeletionQueue {
    pub fn new(current_frame: Arc<AtomicU64>, frames_in_flight: usize) -> Self {
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
            death_frame: self.current_frame.load(Ordering::Relaxed) + self.frames_in_flight as u64,
        });
    }

    /// Call once per frame. Destroys anything safe to destroy.
    pub fn flush(&mut self, device: &ash::Device, allocator: &mut Allocator, current_frame: u64) {
        self.pending.retain_mut(|item| {
            if current_frame >= item.death_frame {
                if let Some(garbage) = item.garbage.take() {
                    garbage.destroy(device, allocator);
                }
                false
            } else {
                true
            }
        });
    }

    /// Call on shutdown — destroys everything regardless of frame.
    pub fn flush_all(&mut self, device: &ash::Device, allocator: &mut Allocator) {
        for mut item in self.pending.drain(..) {
            if let Some(garbage) = item.garbage.take() {
                garbage.destroy(device, allocator);
            }
        }
    }
}

impl Garbage {
    fn destroy(self, device: &ash::Device, allocator: &mut Allocator) {
        unsafe {
            match self {
                Self::Allocation(allocation) => allocator
                    .free(allocation)
                    .expect("Failed to free allocation"),
                Self::Buffer(buffer) => device.destroy_buffer(buffer, None),
                Self::Image(image) => device.destroy_image(image, None),
                Self::ImageView(view) => device.destroy_image_view(view, None),
                Self::Sampler(s) => device.destroy_sampler(s, None),
                Self::Pipeline(p) => device.destroy_pipeline(p, None),
                Self::DescriptorPool(pool) => device.destroy_descriptor_pool(pool, None),
                Self::DescriptorSetLayout(layout) => {
                    device.destroy_descriptor_set_layout(layout, None)
                }
                Self::Semaphore(semaphore) => device.destroy_semaphore(semaphore, None),
                // TODO: actually destroy
                Self::Surface(..) | Self::Swapchain(..) => {}
            }
        }
    }
}
