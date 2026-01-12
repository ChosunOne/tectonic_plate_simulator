use bevy::{
    app::{App, Plugin},
    ecs::{query::With, schedule::IntoScheduleConfigs, world::World},
    log::debug,
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
        DivergenceBindGroup, SwappableBindGroup, TopologyBindGroup, VertexDivergenceBindGroup,
        VertexDivergenceReductionBindGroup, compute_pass::ComputePass,
    },
    plugins::divergence::{DivergencePassLabel, setup_divergence},
    resources::mesh_grid::MeshGrid,
};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, RenderLabel)]
pub struct VertexDivergencePassLabel;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, RenderLabel)]
pub struct VertexDivergenceReductionPassLabel;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct VertexDivergencePlugin;

impl Plugin for VertexDivergencePlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(
            RenderStartup,
            setup_vertex_divergence.after(setup_divergence),
        );
    }
}

fn setup_vertex_divergence(world: &mut World) {
    debug!("Setup vertex divergence");
    let grid = world.resource::<MeshGrid>();
    let num_vertices = grid.vertex_cell_adjacency().len();

    let vertex_divergence_data = vec![0.0f32; num_vertices];
    let num_workgroups = (num_vertices as u32).div_ceil(64);

    let vertex_divergence_pass = ComputePass::builder()
        .label("vertex_divergence")
        .shader("shaders/vertex_divergence.wgsl")
        .workgroups(num_workgroups, 1, 1)
        .buffer(
            vertex_divergence_data,
            false,
            BufferUsages::STORAGE | BufferUsages::COPY_SRC,
            ShaderStages::COMPUTE | ShaderStages::VERTEX,
        )
        .owned_bind_group_marker(VertexDivergenceBindGroup)
        .bind_group::<DivergenceBindGroup>(1)
        .bind_group::<TopologyBindGroup>(2)
        .build(world);

    let vertex_divergence_buffer = {
        let mut query =
            world.query_filtered::<&SwappableBindGroup, With<VertexDivergenceBindGroup>>();
        let bind_group = query
            .single(world)
            .expect("failed to get vertex divergence bind group");
        bind_group
            .get_buffer(0)
            .expect("Failed to get vertex divergence buffer handle")
            .clone()
    };

    let render_device = world.resource::<RenderDevice>();
    let mut builder = SwappableBindGroup::builder();

    builder.add_compute_read(vertex_divergence_buffer);
    builder.add_buffer_data(
        &[f32::MAX, f32::MIN],
        render_device,
        Some("vertex_divergence_bounds"),
        ShaderStages::COMPUTE | ShaderStages::VERTEX,
        BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        false,
    );

    let vertex_divergence_reduction_bind_group = builder.build(
        render_device,
        Some("vertex_divergence_reduction_bind_group"),
    );
    world.spawn((
        vertex_divergence_reduction_bind_group,
        VertexDivergenceReductionBindGroup,
    ));

    let reduction_pass = ComputePass::builder()
        .label("vertex_divergence_reduction")
        .shader("shaders/vertex_divergence_reduction.wgsl")
        .workgroups(1, 1, 1)
        .bind_group::<VertexDivergenceReductionBindGroup>(0)
        .build(world);

    let mut render_graph = world.resource_mut::<RenderGraph>();

    render_graph.add_node(VertexDivergencePassLabel, vertex_divergence_pass);
    render_graph.add_node(VertexDivergenceReductionPassLabel, reduction_pass);
    render_graph.add_node_edge(
        VertexDivergencePassLabel,
        VertexDivergenceReductionPassLabel,
    );
    render_graph.add_node_edge(VertexDivergenceReductionPassLabel, CameraDriverLabel);
    render_graph.add_node_edge(DivergencePassLabel, VertexDivergencePassLabel);
}
