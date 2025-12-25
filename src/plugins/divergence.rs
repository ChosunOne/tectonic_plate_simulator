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
        DivergenceBindGroup, TopologyBindGroup, VelocityBindGroup, compute_pass::ComputePass,
    },
    plugins::phi::PhiPassLabel,
    resources::mantle_grid::MantleGrid,
};

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash, RenderLabel)]
pub struct DivergencePassLabel;

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct DivergencePlugin;

impl Plugin for DivergencePlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(RenderStartup, setup_divergence);
    }
}

fn setup_divergence(world: &mut World) {
    let grid = world.resource::<MantleGrid>();
    let divergence_data = vec![0.0f32; grid.cells().len()];

    let divergence_pass = ComputePass::builder()
        .label("divergence_pass")
        .shader("shaders/divergence.wgsl")
        .buffer_write(divergence_data)
        .workgroups(grid.cells().len().div_ceil(64) as u32, 1, 1)
        .owned_bind_group_marker(DivergenceBindGroup)
        .bind_group::<VelocityBindGroup>(1)
        .bind_group::<TopologyBindGroup>(2)
        .build(world);
    let mut render_graph = world.resource_mut::<RenderGraph>();
    render_graph.add_node(DivergencePassLabel, divergence_pass);
    render_graph.add_node_edge(DivergencePassLabel, PhiPassLabel);
}
