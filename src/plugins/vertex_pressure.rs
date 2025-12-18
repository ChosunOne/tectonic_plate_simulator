use bevy::{
    app::{App, Plugin},
    ecs::world::World,
    render::{
        RenderApp, RenderStartup,
        graph::CameraDriverLabel,
        render_graph::{RenderGraph, RenderLabel},
        render_resource::BufferUsages,
    },
};

use crate::{
    components::render::{PressureBindGroup, VertexPressureBindGroup, compute_pass::ComputePass},
    resources::mantle_grid::MantleGrid,
};

pub struct VertexPressurePlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, RenderLabel)]
pub struct VertexPressurePassLabel;

impl Plugin for VertexPressurePlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(RenderStartup, setup_vertex_pressure);
    }
}

#[allow(clippy::cast_possible_truncation)]
fn setup_vertex_pressure(world: &mut World) {
    let grid = world.resource::<MantleGrid>();
    let adjacency = grid.vertex_cell_adjacency();
    let num_vertices = adjacency.len();

    let vertex_pressure_data = vec![0.0f32; num_vertices];

    let num_workgroups = (num_vertices as u32).div_ceil(64);

    let pass = ComputePass::builder()
        .label("vertex_pressure")
        .shader("shaders/vertex_pressure.wgsl")
        .workgroups(num_workgroups, 1, 1)
        .buffer_read(adjacency.offsets().to_vec())
        .buffer_read(adjacency.indices().to_vec())
        .buffer(
            vertex_pressure_data,
            false,
            BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        )
        .owned_bind_group_marker(VertexPressureBindGroup)
        .bind_group::<PressureBindGroup>(1)
        .build(world);

    let mut render_graph = world.resource_mut::<RenderGraph>();
    render_graph.add_node(VertexPressurePassLabel, pass);
    render_graph.add_node_edge(VertexPressurePassLabel, CameraDriverLabel);
}
