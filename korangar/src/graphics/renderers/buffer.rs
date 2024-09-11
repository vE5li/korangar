use std::collections::Bound;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::num::NonZeroU64;
use std::ops::RangeBounds;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

use bytemuck::{cast_slice, Pod, Zeroable};
#[cfg(feature = "debug")]
use korangar_debug::logging::{print_debug, Colorize};
use wgpu::{BindingResource, BufferDescriptor, BufferSize, BufferSlice, BufferUsages, Device, Queue};

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

        // TODO: NHA We later want to use wgpu's staging belt, but for that we need to
        //       change how we render a scene, since the requirement of the staging to
        //       have access to a command buffer could get hairy. Once we properly use
        //       a bindless approach, that requirement gets much more easier to
        //       fulfill.
        let mut buffer = queue.write_buffer_with(&self.buffer, 0, data_size).unwrap();
        buffer.copy_from_slice(data);
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

    /// Returns the allocated capacity in bytes of the underlying GPU buffer.
    pub fn byte_capacity(&self) -> Option<BufferSize> {
        BufferSize::new(self.capacity)
    }

    /// Returns the number of `T` currently saved inside the buffer.
    pub fn count(&self) -> u32 {
        (self.size.load(Ordering::Acquire) / size_of::<T>() as u64) as u32
    }
}

impl Buffer<u32> {
    /// This function is a special case for the picker, where we want to read
    /// one specific pixel of a special picker buffer (to know where the
    /// cursor is located). This function makes sure that we never stall the
    /// GPU because of this.
    pub fn queue_read_u32(&self, read_index: usize, output: Arc<AtomicU32>) {
        const VALUE_SIZE: usize = size_of::<u32>();

        let captured_buffer = Arc::clone(&self.buffer);
        self.buffer.slice(..).map_async(wgpu::MapMode::Read, move |result| {
            match result {
                Ok(_) => {
                    let mapped = captured_buffer.slice(..).get_mapped_range_mut();
                    let offset = read_index * VALUE_SIZE;

                    if (offset + VALUE_SIZE) <= mapped.len() {
                        // The mapped memory is not guaranteed to be aligned to u32.
                        let mut buffer = [0u8; VALUE_SIZE];
                        buffer.copy_from_slice(&mapped[offset..offset + VALUE_SIZE]);
                        let value = u32::from_ne_bytes(buffer);
                        output.store(value, Ordering::Release)
                    }

                    drop(mapped);
                    captured_buffer.unmap();
                }
                Err(_err) => {
                    #[cfg(feature = "debug")]
                    print_debug!("[{}] failed to map picker buffer: {:?}", "error".red(), _err);
                }
            }
        });
    }
}
