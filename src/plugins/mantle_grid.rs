use bevy::{
    app::{App, Plugin},
    ecs::system::{Commands, Res},
    render::{
        RenderApp, RenderStartup,
        render_resource::{BufferUsages, ShaderStages},
        renderer::RenderDevice,
    },
};

use crate::{
    components::render::{SwappableBindGroup, TopologyBindGroup},
    resources::mantle_grid::MantleGrid,
};

pub struct MantleGridPlugin;

impl Plugin for MantleGridPlugin {
    fn build(&self, app: &mut App) {
        let grid = MantleGrid::new(20);
        app.insert_resource(grid.clone());
        let render_app = app.sub_app_mut(RenderApp);
        render_app.insert_resource(grid);
        render_app.add_systems(RenderStartup, setup_edge_topology);
    }
}

fn setup_edge_topology(
    mut commands: Commands,
    grid: Res<MantleGrid>,
    render_device: Res<RenderDevice>,
) {
    let edge_vertex_adjacency = grid.edge_vertex_adjacency();
    let edge_cell_adjacency = grid.edge_cell_adjacency();
    let cell_edge_adjacency = grid.cell_edge_adjacency();
    let vertex_edge_adjacency = grid.vertex_edge_adjacency();
    let cell_adjacency = grid.cell_adjacency();

    let cell_vertices = grid
        .cells()
        .iter()
        .flat_map(|cell| cell.vertices)
        .collect::<Vec<u32>>();

    let mut builder = SwappableBindGroup::builder();
    let visibility = ShaderStages::COMPUTE;
    let usage = BufferUsages::STORAGE | BufferUsages::COPY_SRC;

    builder.add_buffer_data(
        edge_vertex_adjacency.indices(),
        &render_device,
        Some("edge_vertex_indices"),
        visibility,
        usage,
        true,
    );
    builder.add_buffer_data(
        edge_cell_adjacency.indices(),
        &render_device,
        Some("edge_cell_indices"),
        visibility,
        usage,
        true,
    );
    builder.add_buffer_data(
        cell_edge_adjacency.indices(),
        &render_device,
        Some("cell_edge_indices"),
        visibility,
        usage,
        true,
    );
    builder.add_buffer_data(
        &cell_vertices,
        &render_device,
        Some("cell_vertices"),
        visibility,
        usage,
        true,
    );
    builder.add_buffer_data(
        vertex_edge_adjacency.offsets(),
        &render_device,
        Some("vertex_edge_offsets"),
        visibility,
        usage,
        true,
    );
    builder.add_buffer_data(
        vertex_edge_adjacency.indices(),
        &render_device,
        Some("vertex_edge_indices"),
        visibility,
        usage,
        true,
    );
    builder.add_buffer_data(
        grid.vertex_angle_offsets(),
        &render_device,
        Some("vertex_angle_offsets"),
        ShaderStages::COMPUTE | ShaderStages::VERTEX,
        usage,
        true,
    );
    builder.add_buffer_data(
        cell_adjacency.indices(),
        &render_device,
        Some("cell_cell_indices"),
        visibility,
        usage,
        true,
    );

    let swappable = builder.build(&render_device, Some("topology_bind_group"));
    commands.spawn((swappable, TopologyBindGroup));
}
