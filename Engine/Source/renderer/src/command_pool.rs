use std::{marker::PhantomData, ops::Deref};

use ash::{Device, vk};

use crate::{Queues, Result, physical_device::QueueFamilyIndices};

#[derive(Debug)]
pub struct Graphics;
#[derive(Debug)]
pub struct Transfer;
#[derive(Debug)]
#[allow(unused)]
pub struct Compute;

/// Wrapper for [vk::CommandPool].
pub struct CommandPool<Type: Pool> {
    device: Device,
    /// The command pool
    pool: vk::CommandPool,
    /// The queue commands will be submitted to
    queue: vk::Queue,
    pool_type: PhantomData<Type>,
}

pub trait Pool {
    fn get_index(families: &QueueFamilyIndices) -> u32;
    fn get_queue(queues: &Queues) -> vk::Queue;
}

impl Pool for Compute {
    fn get_index(families: &QueueFamilyIndices) -> u32 {
        families.compute
    }
    fn get_queue(queues: &Queues) -> vk::Queue {
        queues.compute
    }
}

impl Pool for Transfer {
    fn get_index(families: &QueueFamilyIndices) -> u32 {
        families.transfer
    }
    fn get_queue(queues: &Queues) -> vk::Queue {
        queues.transfer
    }
}

impl Pool for Graphics {
    fn get_index(families: &QueueFamilyIndices) -> u32 {
        families.graphics
    }
    fn get_queue(queues: &Queues) -> vk::Queue {
        queues.graphics
    }
}

impl<Type: Pool> CommandPool<Type> {
    /// Will build a command pool with the specified settings.
    /// Please make sure `pool_type` matches the families of the queue
    /// index and the full queue object.
    pub fn build(
        device: &Device,
        queues: &Queues,
        families: &QueueFamilyIndices,
        flags: vk::CommandPoolCreateFlags,
    ) -> Result<Self> {
        let index = Type::get_index(families);
        let queue = Type::get_queue(queues);

        let info = vk::CommandPoolCreateInfo::default()
            .flags(flags)
            .queue_family_index(index);

        let pool = unsafe { device.create_command_pool(&info, None)? };

        Ok(Self {
            device: device.clone(),
            pool,
            queue,
            pool_type: PhantomData,
        })
    }
    pub fn destroy(&self) {
        unsafe {
            self.device.destroy_command_pool(self.pool, None);
        }
    }
    pub fn allocate_buffer(&self, device: &Device) -> Result<CommandBuffer> {
        let allocate_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(self.pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        let buff = unsafe { device.allocate_command_buffers(&allocate_info)?[0] };

        Ok(CommandBuffer {
            buff,
            queue: self.queue,
        })
    }

    pub fn begin_single_time(&self, device: &Device) -> Result<CommandBuffer> {
        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(self.pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let buff = unsafe { device.allocate_command_buffers(&alloc_info)?[0] };

        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe { device.begin_command_buffer(buff, &begin_info)? };
        Ok(CommandBuffer {
            buff,
            queue: self.queue,
        })
    }
}

impl<Type: Pool> Drop for CommandPool<Type> {
    fn drop(&mut self) {
        self.destroy();
    }
}

/// Wrapper for [vk::CommandBuffer].
pub struct CommandBuffer {
    /// The buffer
    buff: vk::CommandBuffer,
    /// The queue to submit to
    queue: vk::Queue,
}

impl CommandBuffer {
    pub fn raw(&self) -> vk::CommandBuffer {
        self.buff
    }
    pub fn submit(
        &self,
        device: &Device,
        submit_info: vk::SubmitInfo,
        fence: vk::Fence,
    ) -> Result<()> {
        unsafe { device.queue_submit(self.queue, std::slice::from_ref(&submit_info), fence)? };
        Ok(())
    }
    pub fn end_and_submit(&self, device: &Device) -> Result<()> {
        let info = vk::SubmitInfo::default().command_buffers(std::slice::from_ref(&self.buff));
        unsafe {
            device.end_command_buffer(self.buff)?;
            device.queue_submit(self.queue, std::slice::from_ref(&info), vk::Fence::null())?;
        };
        Ok(())
    }
}

impl Deref for CommandBuffer {
    type Target = vk::CommandBuffer;

    fn deref(&self) -> &Self::Target {
        &self.buff
    }
}
