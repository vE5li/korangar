use std::marker::PhantomData;
use std::sync::Arc;

use derive_new::new;
use vulkano::buffer::allocator::{SubbufferAllocator, SubbufferAllocatorCreateInfo};
use vulkano::buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::command_buffer::allocator::{CommandBufferAllocator, StandardCommandBufferAllocator};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, CopyBufferInfo, PrimaryCommandBufferAbstract};
use vulkano::descriptor_set::allocator::{DescriptorSetAllocator, StandardDescriptorSetAlloc, StandardDescriptorSetAllocator};
use vulkano::descriptor_set::layout::DescriptorSetLayout;
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::device::{Device, DeviceOwned, Queue};
use vulkano::memory::allocator::{AllocationCreateInfo, AllocationType, MemoryTypeFilter, StandardMemoryAllocator};
use vulkano::memory::{DedicatedAllocation, MemoryRequirements};
use vulkano::pipeline::{GraphicsPipeline, Pipeline, PipelineLayout};
use vulkano::sync::future::FenceSignalFuture;
use vulkano::sync::GpuFuture;
use vulkano::{DeviceSize, VulkanError};

use super::CommandBuilder;

pub struct MemoryAllocator {
    device: Arc<Device>,
    memory_allocator: StandardMemoryAllocator,
    descriptor_set_allocator: StandardDescriptorSetAllocator,
    command_buffer_allocator: StandardCommandBufferAllocator,
}

impl MemoryAllocator {
    pub fn new(device: Arc<Device>) -> Self {
        let memory_allocator = StandardMemoryAllocator::new_default(device.clone());
        let descriptor_set_allocator = StandardDescriptorSetAllocator::new(device.clone());
        let command_buffer_allocator = StandardCommandBufferAllocator::new(device.clone(), Default::default());

        Self {
            device,
            memory_allocator,
            descriptor_set_allocator,
            command_buffer_allocator,
        }
    }
}

unsafe impl DeviceOwned for MemoryAllocator {
    fn device(&self) -> &Arc<vulkano::device::Device> {
        &self.device
    }
}

unsafe impl vulkano::memory::allocator::MemoryAllocator for MemoryAllocator {
    fn find_memory_type_index(&self, memory_type_bits: u32, filter: vulkano::memory::allocator::MemoryTypeFilter) -> Option<u32> {
        self.memory_allocator.find_memory_type_index(memory_type_bits, filter)
    }

    fn allocate_from_type(
        &self,
        memory_type_index: u32,
        create_info: vulkano::memory::allocator::SuballocationCreateInfo,
    ) -> Result<vulkano::memory::allocator::MemoryAlloc, vulkano::memory::allocator::MemoryAllocatorError> {
        self.memory_allocator.allocate_from_type(memory_type_index, create_info)
    }

    unsafe fn allocate_from_type_unchecked(
        &self,
        memory_type_index: u32,
        create_info: vulkano::memory::allocator::SuballocationCreateInfo,
        never_allocate: bool,
    ) -> Result<vulkano::memory::allocator::MemoryAlloc, vulkano::memory::allocator::MemoryAllocatorError> {
        self.memory_allocator
            .allocate_from_type_unchecked(memory_type_index, create_info, never_allocate)
    }

    fn allocate(
        &self,
        requirements: MemoryRequirements,
        allocation_type: AllocationType,
        create_info: AllocationCreateInfo,
        dedicated_allocation: Option<DedicatedAllocation<'_>>,
    ) -> Result<vulkano::memory::allocator::MemoryAlloc, vulkano::memory::allocator::MemoryAllocatorError> {
        self.memory_allocator
            .allocate(requirements, allocation_type, create_info, dedicated_allocation)
    }

    unsafe fn allocate_unchecked(
        &self,
        requirements: MemoryRequirements,
        allocation_type: AllocationType,
        create_info: AllocationCreateInfo,
        dedicated_allocation: Option<DedicatedAllocation<'_>>,
    ) -> Result<vulkano::memory::allocator::MemoryAlloc, vulkano::memory::allocator::MemoryAllocatorError> {
        self.memory_allocator
            .allocate_unchecked(requirements, allocation_type, create_info, dedicated_allocation)
    }

    unsafe fn allocate_dedicated_unchecked(
        &self,
        memory_type_index: u32,
        allocation_size: vulkano::DeviceSize,
        dedicated_allocation: Option<vulkano::memory::DedicatedAllocation>,
        export_handle_types: vulkano::memory::ExternalMemoryHandleTypes,
    ) -> Result<vulkano::memory::allocator::MemoryAlloc, vulkano::memory::allocator::MemoryAllocatorError> {
        self.memory_allocator
            .allocate_dedicated_unchecked(memory_type_index, allocation_size, dedicated_allocation, export_handle_types)
    }
}

unsafe impl DescriptorSetAllocator for MemoryAllocator {
    type Alloc = <StandardDescriptorSetAllocator as DescriptorSetAllocator>::Alloc;

    fn allocate(
        &self,
        layout: &Arc<DescriptorSetLayout>,
        variable_descriptor_count: u32,
    ) -> Result<StandardDescriptorSetAlloc, VulkanError> {
        self.descriptor_set_allocator.allocate(layout, variable_descriptor_count)
    }
}

unsafe impl CommandBufferAllocator for MemoryAllocator {
    type Alloc = <StandardCommandBufferAllocator as CommandBufferAllocator>::Alloc;
    type Builder = <StandardCommandBufferAllocator as CommandBufferAllocator>::Builder;
    type Iter = <StandardCommandBufferAllocator as CommandBufferAllocator>::Iter;

    fn allocate(
        &self,
        queue_family_index: u32,
        level: vulkano::command_buffer::CommandBufferLevel,
        command_buffer_count: u32,
    ) -> Result<Self::Iter, VulkanError> {
        self.command_buffer_allocator
            .allocate(queue_family_index, level, command_buffer_count)
    }
}

#[cfg_attr(feature = "debug", korangar_debug::profile)]
pub(super) fn allocate_descriptor_set(
    pipeline: &Arc<GraphicsPipeline>,
    memory_allocator: &Arc<MemoryAllocator>,
    set_id: u32,
    write_descriptor_sets: impl IntoIterator<Item = WriteDescriptorSet>,
) -> (Arc<PipelineLayout>, Arc<PersistentDescriptorSet>, u32) {
    let layout = pipeline.layout().clone();
    let descriptor_layout = layout.set_layouts().get(set_id as usize).unwrap().clone();
    let set = PersistentDescriptorSet::new(memory_allocator, descriptor_layout, write_descriptor_sets, []).unwrap();

    (layout, set, set_id)
}

pub(super) struct MatrixAllocator<M>
where
    M: BufferContents,
{
    allocator: SubbufferAllocator<Arc<MemoryAllocator>>,
    _matrix_type: PhantomData<M>,
}

impl<M> MatrixAllocator<M>
where
    M: BufferContents,
{
    pub(super) fn new(memory_allocator: &Arc<MemoryAllocator>) -> Self {
        let allocator = SubbufferAllocator::new(memory_allocator.clone(), SubbufferAllocatorCreateInfo {
            arena_size: size_of::<M>() as u64,
            buffer_usage: BufferUsage::UNIFORM_BUFFER,
            memory_type_filter: MemoryTypeFilter::PREFER_HOST | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..Default::default()
        });

        Self {
            allocator,
            _matrix_type: PhantomData,
        }
    }

    pub(super) fn allocate(&self, matrix: M) -> Subbuffer<M> {
        let buffer = self.allocator.allocate_sized::<M>().unwrap();
        *buffer.write().unwrap() = matrix;
        buffer
    }
}

#[derive(new)]
pub struct BufferAllocator {
    memory_allocator: Arc<MemoryAllocator>,
    queue: Arc<Queue>,
    #[new(default)]
    load_buffer: Option<CommandBuilder>,
}

impl BufferAllocator {
    pub fn allocate_vertex_buffer<T, I>(&mut self, data: I) -> Subbuffer<[T]>
    where
        T: BufferContents,
        I: IntoIterator<Item = T>,
        I::IntoIter: ExactSizeIterator,
    {
        self.allocate(data, BufferUsage::VERTEX_BUFFER)
    }

    pub fn allocate_index_buffer<I>(&mut self, data: I) -> Subbuffer<[u16]>
    where
        I: IntoIterator<Item = u16>,
        I::IntoIter: ExactSizeIterator,
    {
        self.allocate(data, BufferUsage::INDEX_BUFFER)
    }

    fn allocate<T, I>(&mut self, data: I, usage: BufferUsage) -> Subbuffer<[T]>
    where
        T: BufferContents,
        I: IntoIterator<Item = T>,
        I::IntoIter: ExactSizeIterator,
    {
        let data = data.into_iter();
        let length = data.len();

        let host_buffer = Buffer::from_iter(
            &*self.memory_allocator,
            BufferCreateInfo {
                usage: BufferUsage::TRANSFER_SRC,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_HOST | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            data,
        )
        .unwrap();

        let device_buffer = Buffer::new_slice(
            &*self.memory_allocator,
            BufferCreateInfo {
                usage: usage | BufferUsage::TRANSFER_DST,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
                ..Default::default()
            },
            length as DeviceSize,
        )
        .unwrap();

        let load_buffer = self.load_buffer.get_or_insert_with(|| {
            AutoCommandBufferBuilder::primary(
                &*self.memory_allocator,
                self.queue.queue_family_index(),
                CommandBufferUsage::OneTimeSubmit,
            )
            .unwrap()
        });

        load_buffer
            .copy_buffer(CopyBufferInfo::buffers(host_buffer, device_buffer.clone()))
            .unwrap();

        device_buffer
    }

    pub fn submit_load_buffer(&mut self) -> Option<FenceSignalFuture<Box<dyn GpuFuture>>> {
        self.load_buffer.take().map(|buffer| {
            buffer
                .build()
                .unwrap()
                .execute(self.queue.clone())
                .unwrap()
                .boxed()
                .then_signal_fence_and_flush()
                .unwrap()
        })
    }
}
