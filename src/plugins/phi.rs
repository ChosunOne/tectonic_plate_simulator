use bevy::{
    app::{App, Plugin},
    ecs::world::World,
    render::{
        RenderApp, RenderStartup,
        render_graph::{RenderGraph, RenderLabel},
    },
};

use crate::{
    components::render::{
        DivergenceBindGroup, PhiBindGroup, TopologyBindGroup, compute_pass::ComputePass,
    },
    plugins::vertex_velocity::VertexVelocityPassLabel,
    resources::mantle_grid::MantleGrid,
};

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash, RenderLabel)]
pub struct PhiPassLabel;

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct PhiPlugin;

impl Plugin for PhiPlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(RenderStartup, setup_phi_iteration);
    }
}

fn setup_phi_iteration(world: &mut World) {
    let grid = world.resource::<MantleGrid>();
    let phi_data = vec![0.0f32; grid.cells().len()];

    let phi_pass = ComputePass::builder()
        .shader("shaders/phi.wgsl")
        .label("phi_pass")
        .workgroups(grid.cells().len().div_ceil(64) as u32, 1, 1)
        .iterations(2)
        .double_buffer(phi_data)
        .owned_bind_group_marker(PhiBindGroup)
        .bind_group::<DivergenceBindGroup>(1)
        .bind_group::<TopologyBindGroup>(2)
        .build(world);

    let mut render_graph = world.resource_mut::<RenderGraph>();
    render_graph.add_node(PhiPassLabel, phi_pass);
    render_graph.add_node_edge(PhiPassLabel, VertexVelocityPassLabel);
}
