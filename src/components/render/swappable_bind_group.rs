use bevy::{
    ecs::component::Component,
    render::{
        render_resource::{
            BindGroup, BindGroupEntry, BindGroupLayout, BindGroupLayoutEntry, BindingType, Buffer,
            BufferBindingType, BufferDescriptor, BufferInitDescriptor, BufferUsages,
            CommandEncoderDescriptor, MapMode, PollType, ShaderStages,
        },
        renderer::{RenderDevice, RenderQueue},
    },
};
use bytemuck::{AnyBitPattern, NoUninit};

use crate::render::double_buffer::{DoubleBuffer, DoubleBufferHandle};

pub enum BufferEntry {
    Static {
        buffer: Buffer,
        visibility: ShaderStages,
        read_only: bool,
    },
    Double {
        double_buffer_index: usize,
        visibility: ShaderStages,
    },
}

#[derive(Default)]
pub struct BindGroupBuilder {
    entries: Vec<BufferEntry>,
    double_buffers: Vec<Box<dyn DoubleBufferHandle + Send + Sync>>,
}

impl BindGroupBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn build(self, render_device: &RenderDevice, label: Option<&str>) -> SwappableBindGroup {
        let mut layout_entries = Vec::new();
        let mut binding_index = 0u32;
        let mut buffers = vec![];

        for entry in &self.entries {
            match entry {
                BufferEntry::Static {
                    visibility,
                    read_only,
                    buffer,
                } => {
                    buffers.push(buffer.clone());
                    layout_entries.push(BindGroupLayoutEntry {
                        binding: binding_index,
                        visibility: *visibility,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage {
                                read_only: *read_only,
                            },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    });
                    binding_index += 1;
                }
                BufferEntry::Double { visibility, .. } => {
                    // Read slot
                    layout_entries.push(BindGroupLayoutEntry {
                        binding: binding_index,
                        visibility: *visibility,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    });
                    binding_index += 1;

                    // Write slot
                    layout_entries.push(BindGroupLayoutEntry {
                        binding: binding_index,
                        visibility: *visibility,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    });
                    binding_index += 1;
                }
            }
        }

        let layout = render_device.create_bind_group_layout(label, &layout_entries);
        let label_a = label.map(|l| format!("{l}_a"));
        let unswapped_bind_group =
            self.build_bind_group(render_device, &layout, label_a.as_deref(), false);
        let bind_groups = if self.double_buffers.is_empty() {
            vec![unswapped_bind_group]
        } else {
            let label_b = label.map(|l| format!("{l}_b"));
            let swapped_bind_group =
                self.build_bind_group(render_device, &layout, label_b.as_deref(), true);
            vec![unswapped_bind_group, swapped_bind_group]
        };

        SwappableBindGroup {
            buffers,
            layout,
            bind_groups,
            current_index: 0,
            double_buffers: self.double_buffers,
        }
    }

    pub fn add_buffer_data<T: 'static + NoUninit + AnyBitPattern + Send + Sync>(
        &mut self,
        data: &[T],
        render_device: &RenderDevice,
        label: Option<&str>,
        visibility: ShaderStages,
        usage: BufferUsages,
        read_only: bool,
    ) -> &mut Self {
        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label,
            contents: bytemuck::cast_slice(data),
            usage,
        });
        self.add_buffer(buffer, visibility, read_only)
    }

    pub fn add_buffer(
        &mut self,
        buffer: Buffer,
        visibility: ShaderStages,
        read_only: bool,
    ) -> &mut Self {
        self.entries.push(BufferEntry::Static {
            buffer,
            visibility,
            read_only,
        });
        self
    }

    pub fn add_double_buffer<T: 'static + NoUninit + AnyBitPattern + Send + Sync>(
        &mut self,
        buffer: DoubleBuffer<T>,
        visibility: ShaderStages,
    ) -> &mut Self {
        let index = self.double_buffers.len();
        self.double_buffers.push(Box::new(buffer));
        self.entries.push(BufferEntry::Double {
            double_buffer_index: index,
            visibility,
        });
        self
    }

    pub fn add_compute(&mut self, buffer: Buffer, read_only: bool) -> &mut Self {
        self.add_buffer(buffer, ShaderStages::COMPUTE, read_only);
        self
    }

    pub fn add_compute_read(&mut self, buffer: Buffer) -> &mut Self {
        self.add_compute(buffer, true)
    }

    pub fn add_compute_write(&mut self, buffer: Buffer) -> &mut Self {
        self.add_compute(buffer, false)
    }

    pub fn add_compute_double<T: 'static + NoUninit + AnyBitPattern + Send + Sync>(
        &mut self,
        double_buffer: DoubleBuffer<T>,
    ) -> &mut Self {
        self.add_double_buffer(double_buffer, ShaderStages::COMPUTE)
    }

    pub fn add_vertex(&mut self, buffer: Buffer, read_only: bool) -> &mut Self {
        self.add_buffer(buffer, ShaderStages::VERTEX, read_only)
    }

    pub fn add_vertex_read(&mut self, buffer: Buffer) -> &mut Self {
        self.add_vertex(buffer, true)
    }

    pub fn add_vertex_write(&mut self, buffer: Buffer) -> &mut Self {
        self.add_vertex(buffer, false)
    }

    pub fn add_fragment(&mut self, buffer: Buffer, read_only: bool) -> &mut Self {
        self.add_buffer(buffer, ShaderStages::FRAGMENT, read_only)
    }

    pub fn add_fragment_read(&mut self, buffer: Buffer) -> &mut Self {
        self.add_fragment(buffer, true)
    }

    pub fn add_fragment_write(&mut self, buffer: Buffer) -> &mut Self {
        self.add_fragment(buffer, false)
    }

    pub fn add_fragment_double<T: 'static + NoUninit + AnyBitPattern + Send + Sync>(
        &mut self,
        double_buffer: DoubleBuffer<T>,
    ) -> &mut Self {
        let index = self.double_buffers.len();
        self.double_buffers.push(Box::new(double_buffer));
        self.entries.push(BufferEntry::Double {
            double_buffer_index: index,
            visibility: ShaderStages::FRAGMENT,
        });
        self
    }

    fn build_bind_group(
        &self,
        render_device: &RenderDevice,
        layout: &BindGroupLayout,
        label: Option<&str>,
        swapped: bool,
    ) -> BindGroup {
        let mut entries = Vec::new();
        let mut binding_index = 0u32;

        for entry in &self.entries {
            match entry {
                BufferEntry::Static { buffer, .. } => {
                    entries.push(BindGroupEntry {
                        binding: binding_index,
                        resource: buffer.as_entire_binding(),
                    });
                    binding_index += 1;
                }
                BufferEntry::Double {
                    double_buffer_index,
                    ..
                } => {
                    let db = &self.double_buffers[*double_buffer_index];
                    let (read_buf, write_buf) = if swapped {
                        (db.write(), db.read())
                    } else {
                        (db.read(), db.write())
                    };
                    entries.push(BindGroupEntry {
                        binding: binding_index,
                        resource: read_buf.as_entire_binding(),
                    });
                    binding_index += 1;
                    entries.push(BindGroupEntry {
                        binding: binding_index,
                        resource: write_buf.as_entire_binding(),
                    });
                    binding_index += 1;
                }
            }
        }

        render_device.create_bind_group(label, layout, &entries)
    }
}

#[derive(Component)]
pub struct SwappableBindGroup {
    layout: BindGroupLayout,
    bind_groups: Vec<BindGroup>,
    pub current_index: usize,
    buffers: Vec<Buffer>,
    double_buffers: Vec<Box<dyn DoubleBufferHandle + Send + Sync>>,
}

impl SwappableBindGroup {
    #[must_use]
    pub fn builder() -> BindGroupBuilder {
        BindGroupBuilder::new()
    }

    /// Gets the layout of the swappable bind group
    #[must_use]
    pub fn layout(&self) -> &BindGroupLayout {
        &self.layout
    }

    /// Gets the version of the bind group corresponding to the current
    /// swap status.
    #[must_use]
    pub fn current(&self) -> &BindGroup {
        &self.bind_groups[self.current_index]
    }

    #[must_use]
    pub fn previous(&self) -> &BindGroup {
        &self.bind_groups[1 - self.current_index]
    }

    /// Swaps the bind groups and the related buffers
    pub fn swap(&mut self) {
        if self.bind_groups.len() <= 1 {
            return;
        }
        self.current_index = 1 - self.current_index;
        for db in &self.double_buffers {
            db.swap();
        }
    }

    /// Get the static buffer indicated by the index. The index in this case is relative
    /// to the total number of *static* buffers, not total buffers.
    #[must_use]
    pub fn get_buffer(&self, index: usize) -> Option<&Buffer> {
        self.buffers.get(index)
    }

    /// Get the double buffer indicated by the index. The index in this case is relative
    /// to the total number of *double* buffers, not total buffers.
    #[must_use]
    pub fn get_double_buffer(
        &self,
        index: usize,
    ) -> Option<&(dyn DoubleBufferHandle + Send + Sync + 'static)> {
        self.double_buffers.get(index).map(|x| &**x)
    }

    /// Reads back the contents of the buffer at the given index.
    #[must_use]
    pub fn read_back_buffer<T: NoUninit + AnyBitPattern + Send + Sync>(
        &self,
        index: usize,
        size: usize,
        render_device: &RenderDevice,
        render_queue: &RenderQueue,
    ) -> Option<Vec<T>> {
        self.buffers
            .get(index)
            .map(|b| {
                let staging_buffer = render_device.create_buffer(&BufferDescriptor {
                    label: Some("readback_buffer"),
                    size: size as u64,
                    usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                let mut encoder = render_device.create_command_encoder(&CommandEncoderDescriptor {
                    label: Some("readback_encoder"),
                });
                encoder.copy_buffer_to_buffer(b, 0, &staging_buffer, 0, size as u64);
                render_queue.submit(std::iter::once(encoder.finish()));

                let slice = staging_buffer.slice(..);
                slice.map_async(MapMode::Read, |_| {});
                let _ = render_device.poll(PollType::Wait);

                let data = slice.get_mapped_range();
                let result = data.to_vec();
                drop(data);
                staging_buffer.unmap();
                result
            })
            .map(|data| bytemuck::cast_slice(&data).to_vec())
    }

    /// Reads back the contents of read buffer of the double buffer at the given index.
    #[must_use]
    pub fn read_back_double_buffer_read<T: NoUninit + AnyBitPattern + Send + Sync>(
        &self,
        index: usize,
        render_device: &RenderDevice,
        render_queue: &RenderQueue,
    ) -> Option<Vec<T>> {
        self.double_buffers
            .get(index)
            .map(|db| db.read_back_read_bytes(render_device, render_queue))
            .map(|data| bytemuck::cast_slice(&data).to_vec())
    }

    /// Reads back the contents of the write buffer of the double buffer at the given index.
    #[must_use]
    pub fn read_back_double_buffer_write<T: NoUninit + AnyBitPattern + Send + Sync>(
        &self,
        index: usize,
        render_device: &RenderDevice,
        render_queue: &RenderQueue,
    ) -> Option<Vec<T>> {
        self.double_buffers
            .get(index)
            .map(|db| db.read_back_write_bytes(render_device, render_queue))
            .map(|data| bytemuck::cast_slice(&data).to_vec())
    }
}
