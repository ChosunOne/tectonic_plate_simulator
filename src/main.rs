use bevy::{prelude::*, render::extract_resource::ExtractResourcePlugin};
use bevy_panorbit_camera::PanOrbitCameraPlugin;
use leafwing_input_manager::{plugin::InputManagerPlugin, prelude::ActionState};
use tectonic_plate_simulator::{
    plugins::{
        mantle_grid::MantleGridPlugin, pressure::PressurePlugin,
        swappable_bind_group::SwappableBindGroupPlugin,
    },
    resources::{gizmo_visibility::GizmoVisibility, mantle_grid::MantleGrid},
    systems::{
        gizmos::{draw_triangle_grid, draw_triangle_grid_centers, draw_triangle_grid_neighbors},
        input::{GizmoAction, gizmo_input_map, toggle_gizmo_visibility},
        setup::setup,
    },
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(InputManagerPlugin::<GizmoAction>::default())
        .add_plugins(PanOrbitCameraPlugin)
        .add_plugins(SwappableBindGroupPlugin)
        .add_plugins(MantleGridPlugin)
        .add_plugins(PressurePlugin)
        .init_resource::<GizmoVisibility>()
        .init_resource::<ActionState<GizmoAction>>()
        .insert_resource(gizmo_input_map())
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                toggle_gizmo_visibility,
                draw_triangle_grid,
                draw_triangle_grid_centers,
                draw_triangle_grid_neighbors,
            ),
        )
        .run();
}
