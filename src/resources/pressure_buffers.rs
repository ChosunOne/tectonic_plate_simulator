use bevy::{
    prelude::*,
    render::{
        render_resource::{Buffer, BufferInitDescriptor, BufferUsages},
        renderer::RenderDevice,
    },
};

use crate::resources::mantle_grid::MantleGrid;
const MAX_TRIANGLES_PER_VERTEX: usize = 8;

#[derive(Resource)]
pub struct PressureBuffers {
    pub pressure_buffer_a: Buffer,
    pub pressure_buffer_b: Buffer,
    pub neighbors_buffer: Buffer,
    pub vertex_triangles_buffer: Buffer,
    pub num_cells: u32,
    pub num_vertices: u32,
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
    let num_vertices = grid.sphere.raw_points().len() as u32;

    let pressures: Vec<f32> = grid.cells.iter().map(|c| c.pressure).collect();
    let neighbors: Vec<u32> = grid
        .neighbors
        .iter()
        .flat_map(|n| n.iter().map(|&idx| idx as u32))
        .collect();

    let mut vertex_triangles_flat = Vec::new();
    for vertex_triangles in &grid.vertex_triangles {
        for i in 0..MAX_TRIANGLES_PER_VERTEX {
            if i < vertex_triangles.len() {
                vertex_triangles_flat.push(vertex_triangles[i] as u32);
            } else {
                vertex_triangles_flat.push(u32::MAX);
            }
        }
    }

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

    let vertex_triangles_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("vertex_triangles_buffer"),
        contents: bytemuck::cast_slice(&vertex_triangles_flat),
        usage: BufferUsages::STORAGE,
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
        vertex_triangles_buffer,
        num_cells,
        num_vertices,
        current_read: true,
    });
}
