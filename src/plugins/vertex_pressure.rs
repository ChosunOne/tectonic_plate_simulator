use bevy::{
    app::{App, Plugin},
    ecs::{query::With, world::World},
    render::{
        RenderApp, RenderStartup,
        graph::CameraDriverLabel,
        render_graph::{RenderGraph, RenderLabel},
        render_resource::{BufferUsages, ShaderStages},
        renderer::RenderDevice,
    },
};

use crate::{
    components::render::{
        PressureBindGroup, SwappableBindGroup, VertexPressureBindGroup,
        VertexPressureReductionBindGroup, compute_pass::ComputePass,
    },
    resources::mantle_grid::MantleGrid,
};

pub struct VertexPressurePlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, RenderLabel)]
pub struct VertexPressurePassLabel;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, RenderLabel)]
pub struct VertexPressureReductionPassLabel;

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

    let vertex_pressure_pass = ComputePass::builder()
        .label("vertex_pressure")
        .shader("shaders/vertex_pressure.wgsl")
        .workgroups(num_workgroups, 1, 1)
        .buffer_read(adjacency.offsets().to_vec())
        .buffer_read(adjacency.indices().to_vec())
        .buffer(
            vertex_pressure_data,
            false,
            BufferUsages::STORAGE | BufferUsages::COPY_SRC,
            ShaderStages::COMPUTE | ShaderStages::VERTEX,
        )
        .owned_bind_group_marker(VertexPressureBindGroup)
        .bind_group::<PressureBindGroup>(1)
        .build(world);

    let vertex_pressure_buffer = {
        let mut query =
            world.query_filtered::<&SwappableBindGroup, With<VertexPressureBindGroup>>();
        let bind_group = query
            .single(world)
            .expect("Failed to get vertex pressure bind group");
        bind_group
            .get_buffer(2)
            .expect("Failed to get vertex pressure buffer handle")
            .clone()
    };

    let render_device = world.resource::<RenderDevice>();
    let mut builder = SwappableBindGroup::builder();
    builder.add_compute_read(vertex_pressure_buffer);
    builder.add_buffer_data(
        &[f32::MAX, f32::MIN],
        render_device,
        Some("vertex_pressure_bounds"),
        ShaderStages::COMPUTE | ShaderStages::VERTEX,
        BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        false,
    );
    let vertex_pressure_reduction_bind_group =
        builder.build(render_device, Some("vertex_pressure_reduction_bind_group"));
    world.spawn((
        vertex_pressure_reduction_bind_group,
        VertexPressureReductionBindGroup,
    ));

    let reduction_pass = ComputePass::builder()
        .label("vertex_pressure_reduction")
        .shader("shaders/vertex_pressure_reduction.wgsl")
        .workgroups(1, 1, 1)
        .bind_group::<VertexPressureReductionBindGroup>(0)
        .build(world);

    let mut render_graph = world.resource_mut::<RenderGraph>();
    render_graph.add_node(VertexPressurePassLabel, vertex_pressure_pass);
    render_graph.add_node(VertexPressureReductionPassLabel, reduction_pass);

    render_graph.add_node_edge(VertexPressurePassLabel, VertexPressureReductionPassLabel);
    render_graph.add_node_edge(VertexPressureReductionPassLabel, CameraDriverLabel);
}
