use std::f32::consts::TAU;

use bevy::{
    app::{App, Plugin},
    ecs::system::{Commands, Res},
    render::{RenderApp, RenderStartup, renderer::RenderDevice},
};

use crate::{
    components::render::{SwappableBindGroup, VelocityBindGroup},
    render::double_buffer::DoubleBuffer,
    resources::mantle_grid::MantleGrid,
};

pub struct VelocityPlugin;

impl Plugin for VelocityPlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(RenderStartup, setup_velocity);
    }
}

fn setup_velocity(mut commands: Commands, grid: Res<MantleGrid>, render_device: Res<RenderDevice>) {
    let num_edges = grid.edge_cell_adjacency().len();
    let mut velocity_data = Vec::with_capacity(num_edges);
    for edge_idx in 0..num_edges {
        let magnitude = edge_idx as f32;
        let angle = (edge_idx as f32 / 10000.0) % TAU;
        velocity_data.push([magnitude, angle]);
    }

    let velocity_buffer =
        DoubleBuffer::new(&render_device, &velocity_data, Some("velocity_buffer"));

    let mut builder = SwappableBindGroup::builder();
    builder.add_compute_double(velocity_buffer);
    let swappable = builder.build(&render_device, Some("velocity_bind_group"));

    commands.spawn((swappable, VelocityBindGroup));
}
