use bevy::{
    app::{App, Plugin},
    ecs::world::World,
    log::debug,
    render::{
        RenderApp, RenderStartup,
        render_graph::{RenderGraph, RenderLabel},
    },
};

use crate::{
    components::render::{
        PressureBindGroup, TopologyBindGroup, VelocityBindGroup, compute_pass::ComputePass,
    },
    resources::mantle_grid::MantleGrid,
};

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash, RenderLabel)]
pub struct VelocityPressurePassLabel;

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct VelocityPressurePlugin;

impl Plugin for VelocityPressurePlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(RenderStartup, setup_velocity_pressure);
    }
}

pub fn setup_velocity_pressure(world: &mut World) {
    debug!("Setup velocity pressure");
    let grid = world.resource::<MantleGrid>();
    let num_edges = grid.edge_cell_adjacency().len();

    let velocity_pressure_pass = ComputePass::builder()
        .label("velocity_pressure_pass")
        .shader("shaders/velocity_pressure.wgsl")
        .workgroups(num_edges.div_ceil(64) as u32, 1, 1)
        .bind_group::<VelocityBindGroup>(0)
        .bind_group::<PressureBindGroup>(1)
        .bind_group::<TopologyBindGroup>(2)
        .build(world);

    let mut render_graph = world.resource_mut::<RenderGraph>();
    render_graph.add_node(VelocityPressurePassLabel, velocity_pressure_pass);
}
