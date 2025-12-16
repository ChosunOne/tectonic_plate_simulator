use bevy::{
    ecs::component::Component,
    render::{
        render_resource::{
            BindGroup, BindGroupEntry, BindGroupLayout, BindGroupLayoutEntry, BindingType, Buffer,
            BufferBindingType, ShaderStages,
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

        for entry in &self.entries {
            match entry {
                BufferEntry::Static {
                    visibility,
                    read_only,
                    ..
                } => {
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
            layout,
            bind_groups,
            current_index: 0,
            double_buffers: self.double_buffers,
        }
    }

    pub fn add_compute_read(&mut self, buffer: Buffer) -> &mut Self {
        self.entries.push(BufferEntry::Static {
            buffer,
            visibility: ShaderStages::COMPUTE,
            read_only: true,
        });
        self
    }

    pub fn add_compute_write(&mut self, buffer: Buffer) -> &mut Self {
        self.entries.push(BufferEntry::Static {
            buffer,
            visibility: ShaderStages::COMPUTE,
            read_only: false,
        });
        self
    }

    pub fn add_compute_double<T: 'static + NoUninit + AnyBitPattern + Send + Sync>(
        &mut self,
        double_buffer: DoubleBuffer<T>,
    ) -> &mut Self {
        let index = self.double_buffers.len();
        self.double_buffers.push(Box::new(double_buffer));
        self.entries.push(BufferEntry::Double {
            double_buffer_index: index,
            visibility: ShaderStages::COMPUTE,
        });
        self
    }

    pub fn add_vertex_read(&mut self, buffer: Buffer) -> &mut Self {
        self.entries.push(BufferEntry::Static {
            buffer,
            visibility: ShaderStages::VERTEX,
            read_only: true,
        });
        self
    }

    pub fn add_vertex_write(&mut self, buffer: Buffer) -> &mut Self {
        self.entries.push(BufferEntry::Static {
            buffer,
            visibility: ShaderStages::VERTEX,
            read_only: false,
        });
        self
    }

    pub fn add_fragment_read(&mut self, buffer: Buffer) -> &mut Self {
        self.entries.push(BufferEntry::Static {
            buffer,
            visibility: ShaderStages::FRAGMENT,
            read_only: true,
        });
        self
    }

    pub fn add_fragment_write(&mut self, buffer: Buffer) -> &mut Self {
        self.entries.push(BufferEntry::Static {
            buffer,
            visibility: ShaderStages::FRAGMENT,
            read_only: false,
        });
        self
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
