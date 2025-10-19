use bevy::prelude::*;
use bevy_panorbit_camera::PanOrbitCameraPlugin;
use tectonic_plate_simulator::systems::{gizmos::draw_triangle_grid, setup::setup};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PanOrbitCameraPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, draw_triangle_grid)
        .run();
}
