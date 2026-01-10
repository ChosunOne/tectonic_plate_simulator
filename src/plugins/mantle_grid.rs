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
    constants::SPHERE_RADIUS,
    resources::mantle_grid::MantleGrid,
};

pub struct MantleGridPlugin;

impl Plugin for MantleGridPlugin {
    fn build(&self, app: &mut App) {
        let grid = MantleGrid::new(100);
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
    debug!("Setup edge topology");
    let edge_vertex_adjacency = grid.edge_vertex_adjacency();
    let edge_cell_adjacency = grid.edge_cell_adjacency();
    let cell_edge_adjacency = grid.cell_edge_adjacency();
    let vertex_edge_adjacency = grid.vertex_edge_adjacency();
    let cell_adjacency = grid.cell_adjacency();
    let vertex_cell_adjacency = grid.vertex_cell_adjacency();

    let cell_vertices = grid
        .cells()
        .iter()
        .flat_map(|cell| cell.vertices)
        .collect::<Vec<u32>>();

    let mut edge_lengths = vec![0.0f32; edge_cell_adjacency.len()];
    for (i, length) in edge_lengths.iter_mut().enumerate() {
        let left_vertex_idx = edge_vertex_adjacency.indices()[i * 2] as usize;
        let right_vertex_idx = edge_vertex_adjacency.indices()[i * 2 + 1] as usize;
        let left_vertex = grid.sphere().raw_points()[left_vertex_idx] * SPHERE_RADIUS;
        let right_vertex = grid.sphere().raw_points()[right_vertex_idx] * SPHERE_RADIUS;
        *length = left_vertex.distance(right_vertex);
    }

    let mut edge_centroid_distance = vec![0.0f32; edge_cell_adjacency.len()];
    for (i, distance) in edge_centroid_distance.iter_mut().enumerate() {
        let primary_cell = grid
            .edge_cell_adjacency()
            .get(i)
            .next()
            .expect("to have a primary cell");
        let secondary_cell = grid
            .edge_cell_adjacency()
            .get(i)
            .nth(1)
            .expect("to have a secondary cell");
        let primary_edges = grid
            .cell_edge_adjacency()
            .get(primary_cell)
            .collect::<Vec<_>>();
        let secondary_edges = grid
            .cell_edge_adjacency()
            .get(secondary_cell)
            .collect::<Vec<_>>();

        let primary_area = cell_area(&primary_edges, &edge_lengths);
        let secondary_area = cell_area(&secondary_edges, &edge_lengths);

        let h1 = 2.0 * primary_area / edge_lengths[i];
        let h2 = 2.0 * secondary_area / edge_lengths[i];
        *distance = (h1 + h2) / 3.0;
    }

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
    builder.add_buffer_data(
        vertex_cell_adjacency.offsets(),
        &render_device,
        Some("vertex_cell_offsets"),
        visibility,
        usage,
        true,
    );
    builder.add_buffer_data(
        vertex_cell_adjacency.indices(),
        &render_device,
        Some("vertex_cell_indices"),
        visibility,
        usage,
        true,
    );
    builder.add_buffer_data(
        &edge_lengths,
        &render_device,
        Some("edge_lengths"),
        visibility,
        usage,
        true,
    );
    builder.add_buffer_data(
        &edge_centroid_distance,
        &render_device,
        Some("edge_centroid_distance"),
        visibility,
        usage,
        true,
    );

    let swappable = builder.build(&render_device, Some("topology_bind_group"));
    commands.spawn((swappable, TopologyBindGroup));
}

fn cell_area(edges: &[usize], edge_lengths: &[f32]) -> f32 {
    let a = edge_lengths[edges[0]];
    let b = edge_lengths[edges[1]];
    let c = edge_lengths[edges[2]];

    let s = (a + b + c) / 2.0;
    (s * (s - a) * (s - b) * s - c).sqrt()
}
