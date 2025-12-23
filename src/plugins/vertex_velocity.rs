use bevy::{
    app::{App, Plugin},
    ecs::{
        query::With,
        schedule::IntoScheduleConfigs,
        system::{Query, Res},
        world::World,
    },
    render::{
        Render, RenderApp, RenderStartup, RenderSystems,
        graph::CameraDriverLabel,
        render_graph::{RenderGraph, RenderLabel},
        render_resource::{BufferUsages, ShaderStages},
        renderer::{RenderDevice, RenderQueue},
    },
};

use crate::{
    components::render::{
        SwappableBindGroup, TopologyBindGroup, VelocityBindGroup, VertexVelocityBindGroup,
        VertexVelocityReductionBindGroup, compute_pass::ComputePass,
    },
    plugins::swappable_bind_group::swap_bind_groups,
    resources::{mantle_grid::MantleGrid, vertex_velocity::VertexVelocitySync},
};

pub struct VertexVelocityPlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, RenderLabel)]
pub struct VertexVelocityPassLabel;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, RenderLabel)]
pub struct VertexVelocityReductionPassLabel;

impl Plugin for VertexVelocityPlugin {
    fn build(&self, app: &mut App) {
        let vertex_velocity_sync = VertexVelocitySync::default();
        app.insert_resource(vertex_velocity_sync.clone());
        let render_app = app.sub_app_mut(RenderApp);
        render_app.insert_resource(vertex_velocity_sync);
        render_app.add_systems(RenderStartup, setup_vertex_velocity);
        render_app.add_systems(
            Render,
            sync_vertex_velocity_to_main
                .in_set(RenderSystems::Cleanup)
                .after(swap_bind_groups),
        );
    }
}

fn setup_vertex_velocity(world: &mut World) {
    let grid = world.resource::<MantleGrid>();
    let vertex_edge_adjacency = grid.vertex_edge_adjacency();
    let num_vertices = vertex_edge_adjacency.len();
    let vertex_velocity_data = vec![[0.0f32, 0.0f32]; num_vertices];

    let num_workgroups = (num_vertices as u32).div_ceil(64);

    let vertex_velocity_pass = ComputePass::builder()
        .label("vertex_velocity")
        .shader("shaders/vertex_velocity.wgsl")
        .workgroups(num_workgroups, 1, 1)
        .buffer(
            vertex_velocity_data,
            false,
            BufferUsages::STORAGE | BufferUsages::COPY_SRC,
            ShaderStages::COMPUTE | ShaderStages::VERTEX,
        )
        .owned_bind_group_marker(VertexVelocityBindGroup)
        .bind_group::<VelocityBindGroup>(1)
        .bind_group::<TopologyBindGroup>(2)
        .build(world);

    let vertex_velocity_buffer = {
        let mut query =
            world.query_filtered::<&SwappableBindGroup, With<VertexVelocityBindGroup>>();
        let bind_group = query
            .single(world)
            .expect("Failed to get vertex velocity bind group");
        bind_group
            .get_buffer(0)
            .expect("Failed to get vertex velocity buffer handle")
    };

    let render_device = world.resource::<RenderDevice>();
    let mut builder = SwappableBindGroup::builder();
    builder.add_compute_read(vertex_velocity_buffer.clone());
    builder.add_buffer_data(
        &[f32::MAX, f32::MIN],
        render_device,
        Some("vertex_velocity_bounds"),
        ShaderStages::COMPUTE | ShaderStages::VERTEX,
        BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        false,
    );
    let vertex_velocity_reduction_bind_group =
        builder.build(render_device, Some("vertex_velocity_reduction_bind_group"));
    world.spawn((
        vertex_velocity_reduction_bind_group,
        VertexVelocityReductionBindGroup,
    ));

    let reduction_pass = ComputePass::builder()
        .label("vertex_velocity_reduction")
        .shader("shaders/vertex_velocity_reduction.wgsl")
        .workgroups(1, 1, 1)
        .bind_group::<VertexVelocityReductionBindGroup>(0)
        .build(world);

    let mut render_graph = world.resource_mut::<RenderGraph>();
    render_graph.add_node(VertexVelocityPassLabel, vertex_velocity_pass);
    render_graph.add_node(VertexVelocityReductionPassLabel, reduction_pass);

    render_graph.add_node_edge(VertexVelocityPassLabel, VertexVelocityReductionPassLabel);
    render_graph.add_node_edge(VertexVelocityReductionPassLabel, CameraDriverLabel);
}

fn sync_vertex_velocity_to_main(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    grid: Res<MantleGrid>,
    vertex_velocity_query: Query<&SwappableBindGroup, With<VertexVelocityBindGroup>>,
    vertex_velocity_sync: Res<VertexVelocitySync>,
) {
    let Ok(vertex_velocity_bg) = vertex_velocity_query.single() else {
        return;
    };

    let num_vertices = grid.vertex_edge_adjacency().len();
    let buffer_size = num_vertices * std::mem::size_of::<[f32; 2]>();

    let Some(vertex_velocity) = vertex_velocity_bg.read_back_buffer::<[f32; 2]>(
        0,
        buffer_size,
        &render_device,
        &render_queue,
    ) else {
        return;
    };

    if let Ok(mut sync_data) = vertex_velocity_sync.0.lock() {
        *sync_data = vertex_velocity;
    }
}
