use bevy::{
    app::{App, Plugin},
    ecs::{schedule::IntoScheduleConfigs, world::World},
    log::debug,
    render::{
        RenderApp, RenderStartup,
        render_graph::{RenderGraph, RenderLabel},
    },
};

use crate::{
    components::render::{
        DepartureBindGroup, SimParamsBindGroup, TopologyBindGroup, VelocityBindGroup,
        compute_pass::ComputePass,
    },
    plugins::viscosity::{ViscosityPassLabel, setup_viscosity},
    resources::mantle_grid::MantleGrid,
};

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash, RenderLabel)]
pub struct AdvectionPassLabel;

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct AdvectionPlugin;

impl Plugin for AdvectionPlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(RenderStartup, setup_advection.after(setup_viscosity));
    }
}

pub fn setup_advection(world: &mut World) {
    debug!("Setup advection");
    let grid = world.resource::<MantleGrid>();
    let num_edges = grid.edge_cell_adjacency().len();
    let num_workgroups = (num_edges as u32).div_ceil(64);

    let advection_pass = ComputePass::builder()
        .label("advection")
        .shader("shaders/advection.wgsl")
        .workgroups(num_workgroups, 1, 1)
        .bind_group::<VelocityBindGroup>(0)
        .bind_group::<TopologyBindGroup>(1)
        .bind_group::<SimParamsBindGroup>(2)
        .bind_group::<DepartureBindGroup>(3)
        .build(world);

    let mut render_graph = world.resource_mut::<RenderGraph>();

    render_graph.add_node(AdvectionPassLabel, advection_pass);
    render_graph.add_node_edge(ViscosityPassLabel, AdvectionPassLabel);
}
