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
        SimParamsBindGroup, TopologyBindGroup, VelocityBindGroup, compute_pass::ComputePass,
    },
    resources::mesh_grid::MeshGrid,
};

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash, RenderLabel)]
pub struct ViscosityPassLabel;

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct ViscosityPlugin;

impl Plugin for ViscosityPlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(RenderStartup, setup_viscosity);
    }
}

pub fn setup_viscosity(world: &mut World) {
    debug!("Setup viscosity");
    let grid = world.resource::<MeshGrid>();
    let num_edges = grid.edge_cell_adjacency().rows();
    let num_workgroups = (num_edges as u32).div_ceil(64);

    let viscosity_pass = ComputePass::builder()
        .label("viscosity")
        .shader("shaders/viscosity.wgsl")
        .workgroups(num_workgroups, 1, 1)
        .bind_group::<VelocityBindGroup>(0)
        .bind_group::<TopologyBindGroup>(1)
        .bind_group::<SimParamsBindGroup>(2)
        .build(world);

    let mut render_graph = world.resource_mut::<RenderGraph>();

    render_graph.add_node(ViscosityPassLabel, viscosity_pass);
}
