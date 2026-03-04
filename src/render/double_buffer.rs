use std::{
    marker::PhantomData,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use bevy::render::{
    render_resource::{
        Buffer, BufferDescriptor, BufferInitDescriptor, BufferUsages, CommandEncoderDescriptor,
        MapMode, PollType,
    },
    renderer::{RenderDevice, RenderQueue},
};
use bytemuck::{AnyBitPattern, NoUninit};

#[derive(Clone, Debug)]
pub struct DoubleBuffer<T: Send + Sync> {
    buffers: [Buffer; 2],
    staging: Buffer,
    // NB: Arc<AtomicUsize> because we want clones of this to stay in sync after swapping
    read_index: Arc<AtomicUsize>,
    len: usize,
    _marker: PhantomData<T>,
}

impl<T: NoUninit + AnyBitPattern + Send + Sync> DoubleBuffer<T> {
    #[must_use]
    pub fn new(render_device: &RenderDevice, data: &[T], label: Option<&str>) -> Self {
        let usage = BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC;
        let bytes = bytemuck::cast_slice(data);
        let label_read = label.map(|l| format!("{l}_a"));
        let label_write = label.map(|l| format!("{l}_b"));
        let label_staging = label.map(|l| format!("{l}_staging"));

        let read_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: label_read.as_deref(),
            contents: bytes,
            usage,
        });
        let write_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: label_write.as_deref(),
            contents: bytes,
            usage,
        });
        let staging = render_device.create_buffer(&BufferDescriptor {
            label: label_staging.as_deref(),
            size: bytes.len() as u64,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            buffers: [read_buffer, write_buffer],
            staging,
            read_index: Arc::new(AtomicUsize::new(0)),
            len: data.len(),
            _marker: PhantomData,
        }
    }

    /// Reads back the contents of the current read buffer. Copies from the GPU back to the CPU.
    #[must_use]
    pub fn read_back_read_buffer(
        &self,
        render_device: &RenderDevice,
        render_queue: &RenderQueue,
    ) -> Vec<T> {
        self.read_back_buffer(self.read(), render_device, render_queue)
    }

    /// Reads back the contents of the current write buffer. Copies from the GPU back to the CPU.
    #[must_use]
    pub fn read_back_write_buffer(
        &self,
        render_device: &RenderDevice,
        render_queue: &RenderQueue,
    ) -> Vec<T> {
        self.read_back_buffer(self.write(), render_device, render_queue)
    }

    fn read_back_buffer(
        &self,
        buffer: &Buffer,
        render_device: &RenderDevice,
        render_queue: &RenderQueue,
    ) -> Vec<T> {
        let bytes = self.read_back_buffer_bytes(buffer, render_device, render_queue);
        bytemuck::cast_slice(&bytes).to_vec()
    }

    fn read_back_buffer_bytes(
        &self,
        buffer: &Buffer,
        render_device: &RenderDevice,
        render_queue: &RenderQueue,
    ) -> Vec<u8> {
        let size = (self.len * std::mem::size_of::<T>()) as u64;
        let mut encoder = render_device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("readback_encoder"),
        });
        encoder.copy_buffer_to_buffer(buffer, 0, &self.staging, 0, size);
        render_queue.submit(std::iter::once(encoder.finish()));

        let slice = self.staging.slice(..);
        slice.map_async(MapMode::Read, |_| {});
        let _ = render_device.poll(PollType::Wait {
            timeout: None,
            submission_index: None,
        });

        let data = slice.get_mapped_range();
        let result = data.to_vec();
        drop(data);
        self.staging.unmap();
        result
    }
}

pub trait DoubleBufferHandle {
    fn read(&self) -> &Buffer;
    fn write(&self) -> &Buffer;
    fn swap(&self);
    fn elem_size(&self) -> usize;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn read_back_read_bytes(
        &self,
        render_device: &RenderDevice,
        render_queue: &RenderQueue,
    ) -> Vec<u8>;
    fn read_back_write_bytes(
        &self,
        render_device: &RenderDevice,
        render_queue: &RenderQueue,
    ) -> Vec<u8>;
}

impl<T: NoUninit + AnyBitPattern + Send + Sync> DoubleBufferHandle for DoubleBuffer<T> {
    /// Returns a reference to the current read buffer.
    fn read(&self) -> &Buffer {
        &self.buffers[self.read_index.load(Ordering::Acquire)]
    }

    /// Returns a reference to the write buffer.
    fn write(&self) -> &Buffer {
        &self.buffers[1 - self.read_index.load(Ordering::Acquire)]
    }

    /// Swap the read and write buffers.
    fn swap(&self) {
        let old = self.read_index.load(Ordering::Acquire);
        self.read_index.store(1 - old, Ordering::Release);
    }

    /// The size of each element in the buffer
    fn elem_size(&self) -> usize {
        std::mem::size_of::<T>()
    }

    /// The number of pre-serialization elements in the buffer.
    fn len(&self) -> usize {
        self.len
    }

    /// Whether the buffer is empty.
    fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn read_back_read_bytes(
        &self,
        render_device: &RenderDevice,
        render_queue: &RenderQueue,
    ) -> Vec<u8> {
        self.read_back_buffer_bytes(self.read(), render_device, render_queue)
    }

    fn read_back_write_bytes(
        &self,
        render_device: &RenderDevice,
        render_queue: &RenderQueue,
    ) -> Vec<u8> {
        self.read_back_buffer_bytes(self.write(), render_device, render_queue)
    }
}
