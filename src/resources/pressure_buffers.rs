use bevy::{
    prelude::*,
    render::{
        render_resource::{Buffer, BufferInitDescriptor, BufferUsages},
        renderer::RenderDevice,
    },
};

use crate::resources::mantle_grid::MantleGrid;

#[derive(Resource)]
pub struct PressureBuffers {
    pub pressure_buffer_a: Buffer,
    pub pressure_buffer_b: Buffer,
    pub neighbors_buffer: Buffer,
    pub num_cells: u32,
    pub current_read: bool,
}

pub fn prepare_buffers(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    grid: Res<MantleGrid>,
    buffers: Option<Res<PressureBuffers>>,
) {
    if buffers.is_some() {
        return;
    }

    let num_cells = grid.cells.len() as u32;

    let pressures: Vec<f32> = grid.cells.iter().map(|c| c.pressure).collect();
    let neighbors: Vec<u32> = grid
        .neighbors
        .iter()
        .flat_map(|n| n.iter().map(|&idx| idx as u32))
        .collect();

    let pressure_buffer_a = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("pressure_buffer"),
        contents: bytemuck::cast_slice(&pressures),
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
    });

    let pressure_buffer_b = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("pressure_buffer"),
        contents: bytemuck::cast_slice(&pressures),
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
    });

    let neighbors_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("neighbors_buffer"),
        contents: bytemuck::cast_slice(&neighbors),
        usage: BufferUsages::STORAGE,
    });

    commands.insert_resource(PressureBuffers {
        pressure_buffer_a,
        pressure_buffer_b,
        neighbors_buffer,
        num_cells,
        current_read: true,
    });
}
