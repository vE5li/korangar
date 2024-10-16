use std::collections::Bound;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::num::NonZeroU64;
use std::ops::RangeBounds;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use bytemuck::{Pod, Zeroable, bytes_of, cast_slice};
#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, print_debug};
use wgpu::util::StagingBelt;
use wgpu::{BindingResource, BindingType, BufferBindingType, BufferDescriptor, BufferSlice, BufferUsages, CommandEncoder, Device, Queue};

/// Convenience abstraction over GPU buffers. Can be seen as a "Vec<T>" on the
/// GPU.
pub struct Buffer<T: ?Sized> {
    label: String,
    size: AtomicU64,
    capacity: u64,
    usage: BufferUsages,
    buffer: Arc<wgpu::Buffer>,
    _marker: PhantomData<T>,
}

impl<T> Debug for Buffer<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Buffer(\"{}\")", self.label)
    }
}

impl<T: Sized + Pod + Zeroable> Buffer<T> {
    pub fn with_capacity(device: &Device, label: impl Into<String>, usage: BufferUsages, capacity: u64) -> Self {
        let label = label.into();
        let buffer = Arc::new(device.create_buffer(&BufferDescriptor {
            label: Some(&label),
            size: capacity,
            usage,
            mapped_at_creation: false,
        }));

        Self {
            label,
            size: AtomicU64::new(0),
            capacity,
            usage,
            buffer,
            _marker: PhantomData,
        }
    }

    pub fn with_data(device: &Device, queue: &Queue, label: impl Into<String>, usage: BufferUsages, data: &[T]) -> Self {
        let label = label.into();
        let size = size_of_val(data) as u64;
        let buffer = Arc::new(device.create_buffer(&BufferDescriptor {
            label: Some(&label),
            size,
            usage,
            mapped_at_creation: false,
        }));

        let buffer = Self {
            label,
            size: AtomicU64::new(size),
            capacity: size,
            usage,
            buffer,
            _marker: PhantomData,
        };
        buffer.write_exact(queue, data);

        buffer
    }

    /// Convince function to create [`BindingResource`] from the buffer.
    pub fn as_entire_binding(&self) -> BindingResource<'_> {
        self.buffer.as_entire_binding()
    }

    /// Used when the user wants to write data using the whole buffer capacity.
    ///
    /// # Note
    /// Panics when data is not the exact size as the buffer capacity.
    pub fn write_exact(&self, queue: &Queue, data: &[T]) {
        let data: &[u8] = cast_slice(data);

        let Some(data_size) = NonZeroU64::new(data.len() as u64) else {
            return;
        };

        if self.capacity != data_size.get() {
            panic!(
                "data size and buffer capacity don't match! capacity: {}, data size: {}",
                self.capacity,
                data_size.get()
            );
        }
        self.size.store(data_size.get(), Ordering::Release);
        let mut buffer = queue.write_buffer_with(&self.buffer, 0, data_size).unwrap();
        buffer.copy_from_slice(data);
    }

    /// Used when the user wants to write data. Can re-allocate the buffer if
    /// it's not big enough. Return's `true` if the buffer had to be re-created.
    pub fn write(&mut self, device: &Device, staging_belt: &mut StagingBelt, command_encoder: &mut CommandEncoder, data: &[T]) -> bool {
        let data: &[u8] = cast_slice(data);

        let Some(data_size) = NonZeroU64::new(data.len() as u64) else {
            return false;
        };

        let mut recreated = false;
        if self.capacity < data_size.get() {
            recreated = true;
            let size = data_size.get();
            self.capacity = size;

            self.buffer = Arc::new(device.create_buffer(&BufferDescriptor {
                label: Some(&self.label),
                size,
                usage: self.usage,
                mapped_at_creation: false,
            }));
        }
        self.size.store(data_size.get(), Ordering::Release);

        let mut buffer = staging_belt.write_buffer(command_encoder, &self.buffer, 0, data_size, device);
        buffer.copy_from_slice(data);

        recreated
    }

    /// Reserves the given capacity. Will re-create the buffer if it's too
    /// small. Returns `true` if the buffer had to be re-created.
    pub fn reserve(&mut self, device: &Device, count: usize) -> bool {
        let size = (size_of::<T>() * count) as u64;

        if self.capacity < size {
            self.capacity = size;

            self.buffer = Arc::new(device.create_buffer(&BufferDescriptor {
                label: Some(&self.label),
                size,
                usage: self.usage,
                mapped_at_creation: false,
            }));

            true
        } else {
            false
        }
    }

    /// Returns a reference to the backing GPU buffer.
    pub fn get_buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    /// Returns a sliced view into the buffer.
    pub fn slice(&self, bounds: impl RangeBounds<usize>) -> BufferSlice<'_> {
        let start = match bounds.start_bound() {
            Bound::Included(&start) => (start * size_of::<T>()) as u64,
            Bound::Excluded(&start) => ((start + 1) * size_of::<T>()) as u64,
            Bound::Unbounded => 0,
        };

        let end = match bounds.end_bound() {
            Bound::Included(&end) => ((end + 1) * size_of::<T>()) as u64,
            Bound::Excluded(&end) => (end * size_of::<T>()) as u64,
            Bound::Unbounded => self.size.load(Ordering::Acquire),
        };

        self.buffer.slice(start..end)
    }

    /// Returns the number of `T` currently saved inside the buffer.
    pub fn count(&self) -> u32 {
        (self.size.load(Ordering::Acquire) / size_of::<T>() as u64) as u32
    }
}

impl Buffer<u64> {
    /// This function is a special case for the picker, where we want to read
    /// back the picker value.
    pub fn queue_read_u64(&self, output: Arc<AtomicU64>) {
        const VALUE_SIZE: usize = size_of::<u64>();

        let captured_buffer = Arc::clone(&self.buffer);
        self.buffer.slice(..).map_async(wgpu::MapMode::Read, move |result| {
            match result {
                Ok(_) => {
                    let mapped = captured_buffer.slice(..).get_mapped_range_mut();

                    if VALUE_SIZE <= mapped.len() {
                        // The mapped memory is not guaranteed to be aligned to u64.
                        let mut buffer = [0u8; VALUE_SIZE];
                        buffer.copy_from_slice(&mapped[..VALUE_SIZE]);
                        let value = u64::from_le_bytes(buffer);
                        output.store(value, Ordering::Release)
                    }

                    drop(mapped);
                    captured_buffer.unmap();
                }
                Err(_error) => {
                    #[cfg(feature = "debug")]
                    print_debug!("[{}] failed to map picker buffer: {:?}", "error".red(), _error);
                }
            }
        });
    }
}

pub struct DynamicUniformBuffer<T> {
    buffer: Buffer<u8>,
    data: Vec<u8>,
    aligned_size: usize,
    marker: PhantomData<T>,
}

impl<T: Sized + Pod + Zeroable> DynamicUniformBuffer<T> {
    pub fn new(device: &Device, label: &str) -> Self {
        let uniform_alignment = device.limits().min_uniform_buffer_offset_alignment as usize;
        let aligned_size = (size_of::<T>() + uniform_alignment - 1) & !(uniform_alignment - 1);
        let buffer = Buffer::with_capacity(device, label, BufferUsages::UNIFORM | BufferUsages::COPY_DST, aligned_size as _);

        Self {
            buffer,
            data: Vec::new(),
            aligned_size,
            marker: PhantomData,
        }
    }

    pub fn write_data<D>(&mut self, data: D)
    where
        D: IntoIterator<Item = T>,
    {
        for (index, uniform) in data.into_iter().enumerate() {
            let start = index * self.aligned_size;
            let end = start + size_of::<T>();

            if self.data.len() < end {
                self.data.resize(end, 0);
            }

            self.data[start..end].copy_from_slice(bytes_of(&uniform));
        }
    }

    pub fn upload(&mut self, device: &Device, staging_belt: &mut StagingBelt, command_encoder: &mut CommandEncoder) -> bool {
        self.buffer.write(device, staging_belt, command_encoder, &self.data)
    }

    pub fn dynamic_offset(&self, index: usize) -> u32 {
        (index * self.aligned_size) as u32
    }

    pub fn get_binding_type() -> BindingType {
        BindingType::Buffer {
            ty: BufferBindingType::Uniform,
            has_dynamic_offset: true,
            min_binding_size: NonZeroU64::new(size_of::<T>() as _),
        }
    }

    pub fn get_binding_resource(&self) -> BindingResource<'_> {
        BindingResource::Buffer(wgpu::BufferBinding {
            buffer: self.buffer.get_buffer(),
            offset: 0,
            size: Some(NonZeroU64::new(size_of::<T>() as u64).unwrap()),
        })
    }
}
