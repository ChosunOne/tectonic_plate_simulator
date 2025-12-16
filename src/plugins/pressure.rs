use bevy::{
    app::{App, Plugin},
    ecs::system::{Commands, Res},
    log::debug,
    render::{RenderApp, RenderStartup, renderer::RenderDevice},
};

use crate::{
    components::render::{PressureBindGroup, SwappableBindGroup},
    render::double_buffer::DoubleBuffer,
    resources::mantle_grid::MantleGrid,
};

/// Basic plugin for initializing pressure data
pub struct PressurePlugin;

impl Plugin for PressurePlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(RenderStartup, setup_pressure);
    }
}

fn setup_pressure(mut commands: Commands, grid: Res<MantleGrid>, render_device: Res<RenderDevice>) {
    debug!("Loading pressure data");
    let num_cells = grid.cells().len();
    let mut pressure_data = Vec::with_capacity(num_cells);
    let mut total = 1.0f32;
    for _ in 0..num_cells {
        pressure_data.push(total);
        total += 1.0;
    }

    let pressure_buffer =
        DoubleBuffer::new(&render_device, &pressure_data, Some("pressure_data_buffer"));

    let mut builder = SwappableBindGroup::builder();
    builder.add_compute_double(pressure_buffer);
    let swappable = builder.build(&render_device, Some("pressure_bind_group"));

    commands.spawn((swappable, PressureBindGroup));
    debug!("Pressure data initialized");
}
