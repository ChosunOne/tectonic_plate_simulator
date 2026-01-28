use bevy::{
    app::{App, Plugin},
    ecs::system::{Commands, Res},
    log::debug,
    render::{
        RenderApp, RenderStartup,
        render_resource::{BufferUsages, ShaderStages},
        renderer::RenderDevice,
    },
};

use crate::{
    components::render::{SwappableBindGroup, TopologyBindGroup},
    resources::mesh_grid::MeshGrid,
};

pub struct MeshGridPlugin;

impl Plugin for MeshGridPlugin {
    fn build(&self, app: &mut App) {
        let grid = MeshGrid::new(100);
        app.insert_resource(grid.clone());
        let render_app = app.sub_app_mut(RenderApp);
        render_app.insert_resource(grid);
        render_app.add_systems(RenderStartup, setup_edge_topology);
    }
}

fn setup_edge_topology(
    mut commands: Commands,
    grid: Res<MeshGrid>,
    render_device: Res<RenderDevice>,
) {
    debug!("Setup edge topology");
    let edge_vertex_adjacency = grid.edge_vertex_adjacency();
    let edge_cell_adjacency = grid.edge_cell_adjacency();
    let cell_edge_adjacency = grid.cell_edge_adjacency();
    let vertex_edge_adjacency = grid.vertex_edge_adjacency();
    let cell_adjacency = grid.cell_adjacency();
    let vertex_cell_adjacency = grid.vertex_cell_adjacency();
    let edge_lengths = grid.edge_lengths();
    let edge_centroid_distance = grid.edge_centroid_distance();

    let cell_vertices = grid
        .cells()
        .iter()
        .flat_map(|cell| cell.vertices)
        .collect::<Vec<u32>>();

    let mut builder = SwappableBindGroup::builder();
    let visibility = ShaderStages::COMPUTE;
    let usage = BufferUsages::STORAGE | BufferUsages::COPY_SRC;

    builder.add_buffer_data(
        edge_vertex_adjacency.data(),
        &render_device,
        Some("edge_vertex_indices"),
        visibility,
        usage,
        true,
    );
    builder.add_buffer_data(
        edge_cell_adjacency.data(),
        &render_device,
        Some("edge_cell_indices"),
        visibility,
        usage,
        true,
    );
    builder.add_buffer_data(
        cell_edge_adjacency.data(),
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
        vertex_edge_adjacency.indptr().as_slice().unwrap(),
        &render_device,
        Some("vertex_edge_offsets"),
        visibility,
        usage,
        true,
    );
    builder.add_buffer_data(
        vertex_edge_adjacency.data(),
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
        cell_adjacency.data(),
        &render_device,
        Some("cell_cell_indices"),
        visibility,
        usage,
        true,
    );
    builder.add_buffer_data(
        vertex_cell_adjacency.indptr().as_slice().unwrap(),
        &render_device,
        Some("vertex_cell_offsets"),
        visibility,
        usage,
        true,
    );
    builder.add_buffer_data(
        vertex_cell_adjacency.data(),
        &render_device,
        Some("vertex_cell_indices"),
        visibility,
        usage,
        true,
    );
    builder.add_buffer_data(
        edge_lengths,
        &render_device,
        Some("edge_lengths"),
        visibility,
        usage,
        true,
    );
    builder.add_buffer_data(
        edge_centroid_distance,
        &render_device,
        Some("edge_centroid_distance"),
        visibility,
        usage,
        true,
    );
    builder.add_buffer_data(
        grid.edge_transport_connection(),
        &render_device,
        Some("edge_transport_connection"),
        visibility,
        usage,
        true,
    );

    let swappable = builder.build(&render_device, "topology_bind_group");
    commands.spawn((swappable, TopologyBindGroup));
}
