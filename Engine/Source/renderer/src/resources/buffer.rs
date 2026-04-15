//! This module houses the vulkan buffer abstraction.

use std::ptr::NonNull;
use std::{ffi::c_void, marker::PhantomData};

use ash::{Device, vk};
use gpu_allocator::{
    MemoryLocation,
    vulkan::{Allocation, AllocationCreateDesc},
};

use crate::{Renderer, Result};

/// An abstraction around the vulkan buffer.
pub struct Buffer<Type: BuffType = Custom> {
    device: Device,
    buffer: vk::Buffer,
    allocation: Allocation,
    buffer_type: PhantomData<Type>,
}

pub trait BuffType {
    fn get_usage() -> vk::BufferUsageFlags;
}

macro_rules! define_buff_type {
    ($name:ident, $usage:expr) => {
        pub struct $name;

        impl BuffType for $name {
            fn get_usage() -> vk::BufferUsageFlags {
                $usage
            }
        }

        pastey::paste! {
            pub type [<$name Buffer>] = Buffer<$name>;
        }
    };
}

// flags don't matter
define_buff_type!(Custom, vk::BufferUsageFlags::STORAGE_BUFFER);

impl<Type: BuffType> Buffer<Type> {
    pub fn create_custom(
        renderer: &mut Renderer,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        location: MemoryLocation,
    ) -> Result<Self> {
        let buffer_info = vk::BufferCreateInfo::default()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let buffer = unsafe { renderer.device.create_buffer(&buffer_info, None)? };
        let requirements = unsafe { renderer.device.get_buffer_memory_requirements(buffer) };

        let allocation = renderer.allocator.allocate(&AllocationCreateDesc {
            name: "buffer",
            requirements,
            location,
            linear: true, // buffers are always linear
            allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged,
        })?;

        unsafe {
            renderer
                .device
                .bind_buffer_memory(buffer, allocation.memory(), allocation.offset())?
        };

        Ok(Self {
            device: renderer.device.clone(),
            buffer,
            allocation,
            buffer_type: PhantomData,
        })
    }
    pub fn create(
        renderer: &mut Renderer,
        size: vk::DeviceSize,
        location: MemoryLocation,
    ) -> Result<Self> {
        Buffer::create_custom(renderer, size, Type::get_usage(), location)
    }
    pub fn buffer(&self) -> vk::Buffer {
        self.buffer
    }
    pub fn mapped(&self) -> Option<NonNull<c_void>> {
        self.allocation.mapped_ptr()
    }

    pub fn upload_slice<T: Copy>(renderer: &mut Renderer, data: &[T]) -> Result<Self> {
        let size = std::mem::size_of_val(data) as vk::DeviceSize;

        let staging_buf = Buffer::create_custom(
            renderer,
            size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            MemoryLocation::CpuToGpu,
        )?;

        unsafe {
            let ptr = staging_buf.mapped().unwrap().as_ptr() as *mut T;
            ptr.copy_from_nonoverlapping(data.as_ptr(), data.len());
        }

        let device_buf = Buffer::create_custom(
            renderer,
            size,
            vk::BufferUsageFlags::TRANSFER_DST | Type::get_usage(),
            MemoryLocation::GpuOnly,
        )?;

        device_buf.copy(renderer, &staging_buf, size)?;
        // TODO: destroy buffer when it is no longer needed (maybe with VMA)
        // currently it is destroyed too early, it is still in use
        // unsafe {
        //     self.device.destroy_buffer(staging_buf, None);
        //     self.device.free_memory(staging_mem, None);
        // }

        Ok(device_buf)
    }

    /// Copy src into self
    pub fn copy(&self, renderer: &Renderer, src: &Buffer, size: vk::DeviceSize) -> Result<()> {
        let cmd = renderer.transfer_pool.begin_single_time()?;

        let region = vk::BufferCopy {
            src_offset: 0,
            dst_offset: 0,
            size,
        };
        unsafe {
            self.device
                .cmd_copy_buffer(cmd.raw(), src.buffer(), self.buffer(), &[region])
        };

        cmd.end_and_submit()?;
        Ok(())
    }
}

impl<Type: BuffType> Drop for Buffer<Type> {
    fn drop(&mut self) {
        unsafe { self.device.destroy_buffer(self.buffer, None) };
        // TODO: free the allocation
    }
}

define_buff_type!(Uniform, vk::BufferUsageFlags::UNIFORM_BUFFER);

impl UniformBuffer {
    /// Copies `data` into the persistently-mapped host-visible memory.
    ///
    /// # Safety
    /// The mapped pointer must be valid and the allocation must cover at least
    /// `size_of::<T>()` bytes — both invariants are guaranteed by every
    /// `UboData` constructed in this module.
    pub unsafe fn write<T: Copy>(&self, data: &T) {
        let ptr = self
            .mapped()
            .expect("UBO allocation must be host-visible")
            .as_ptr() as *mut u8;
        unsafe { std::ptr::copy_nonoverlapping(data as *const T as *const u8, ptr, size_of::<T>()) }
    }
}

define_buff_type!(Vertex, vk::BufferUsageFlags::VERTEX_BUFFER);
define_buff_type!(Index, vk::BufferUsageFlags::INDEX_BUFFER);
