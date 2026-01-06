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
        DivergenceBindGroup, PhiBindGroup, PressureBindGroup, SimParamsBindGroup,
        TopologyBindGroup, VelocityBindGroup, compute_pass::ComputePass,
    },
    plugins::advection::{AdvectionPassLabel, setup_advection},
    resources::mantle_grid::MantleGrid,
};

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash, RenderLabel)]
pub struct DivergencePassLabel;

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash, RenderLabel)]
pub struct PhiZeroPassLabel;

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash, RenderLabel)]
pub struct PhiPassLabel;

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash, RenderLabel)]
pub struct PhiPressurePassLabel;

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash, RenderLabel)]
pub struct PhiVelocityPassLabel;

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct DivergencePlugin;

impl Plugin for DivergencePlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(RenderStartup, setup_divergence.after(setup_advection));
    }
}

pub fn setup_divergence(world: &mut World) {
    debug!("Setup divergence plugin");
    let grid = world.resource::<MantleGrid>();
    let divergence_data = vec![0.0f32; grid.cells().len()];
    let phi_data = vec![0.0f32; grid.cells().len()];
    let num_cells = grid.cells().len();
    let num_edges = grid.edge_cell_adjacency().len();

    let divergence_pass = ComputePass::builder()
        .label("divergence_pass")
        .shader("shaders/divergence.wgsl")
        .buffer_write(divergence_data)
        .workgroups(num_cells.div_ceil(64) as u32, 1, 1)
        .owned_bind_group_marker(DivergenceBindGroup)
        .bind_group::<VelocityBindGroup>(1)
        .bind_group::<TopologyBindGroup>(2)
        .bind_group::<SimParamsBindGroup>(3)
        .build(world);

    let phi_zero_pass = ComputePass::builder()
        .shader("shaders/phi_zero.wgsl")
        .label("phi_zero_pass")
        .workgroups(num_cells.div_ceil(64) as u32, 1, 1)
        .iterations(2)
        .swap_each_iter(true)
        .double_buffer(phi_data)
        .owned_bind_group_marker(PhiBindGroup)
        .build(world);

    let phi_pass = ComputePass::builder()
        .shader("shaders/phi.wgsl")
        .label("phi_pass")
        .workgroups(num_cells.div_ceil(64) as u32, 1, 1)
        .iterations(200)
        .swap_each_iter(true)
        .bind_group::<PhiBindGroup>(0)
        .bind_group::<DivergenceBindGroup>(1)
        .bind_group::<TopologyBindGroup>(2)
        .build(world);

    let phi_pressure_pass = ComputePass::builder()
        .shader("shaders/phi_pressure.wgsl")
        .label("phi_pressure_pass")
        .workgroups(num_cells.div_ceil(64) as u32, 1, 1)
        .bind_group::<PressureBindGroup>(0)
        .bind_group::<PhiBindGroup>(1)
        .build(world);

    let phi_velocity_pass = ComputePass::builder()
        .shader("shaders/phi_velocity.wgsl")
        .label("phi_velocity_pass")
        .workgroups(num_edges.div_ceil(64) as u32, 1, 1)
        .bind_group::<VelocityBindGroup>(0)
        .bind_group::<PhiBindGroup>(1)
        .bind_group::<TopologyBindGroup>(2)
        .bind_group::<SimParamsBindGroup>(3)
        .build(world);
    let mut render_graph = world.resource_mut::<RenderGraph>();
    render_graph.add_node(DivergencePassLabel, divergence_pass);
    render_graph.add_node(PhiZeroPassLabel, phi_zero_pass);
    render_graph.add_node(PhiPassLabel, phi_pass);
    render_graph.add_node(PhiPressurePassLabel, phi_pressure_pass);
    render_graph.add_node(PhiVelocityPassLabel, phi_velocity_pass);
    render_graph.add_node_edge(AdvectionPassLabel, DivergencePassLabel);
    render_graph.add_node_edge(DivergencePassLabel, PhiZeroPassLabel);
    render_graph.add_node_edge(PhiZeroPassLabel, PhiPassLabel);
    render_graph.add_node_edge(PhiPassLabel, PhiPressurePassLabel);
    render_graph.add_node_edge(PhiPressurePassLabel, PhiVelocityPassLabel);
}
