use bevy::{prelude::*, render::extract_resource::ExtractResourcePlugin};
use bevy_panorbit_camera::PanOrbitCameraPlugin;
use tectonic_plate_simulator::{
    materials::pressure_material::PressureMaterial,
    plugins::pressure_solver::PressureSolverPlugin,
    resources::vertex_pressure_buffer::VertexPressureBufferHandle,
    systems::{
        gizmos::{draw_triangle_grid, draw_triangle_grid_centers, draw_triangle_grid_neighbors},
        setup::setup,
    },
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PanOrbitCameraPlugin)
        .add_plugins(MaterialPlugin::<PressureMaterial>::default())
        .add_plugins(ExtractResourcePlugin::<VertexPressureBufferHandle>::default())
        .add_plugins(PressureSolverPlugin)
        .add_systems(Startup, setup)
        // .add_systems(
        //     Update,
        //     (
        // draw_triangle_grid,
        // draw_triangle_grid_centers,
        // draw_triangle_grid_neighbors,
        // ),
        // )
        .run();
}
