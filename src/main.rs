use bevy::prelude::*;
use bevy_panorbit_camera::PanOrbitCameraPlugin;
use leafwing_input_manager::{plugin::InputManagerPlugin, prelude::ActionState};
use tectonic_plate_simulator::{
    materials::{pressure_material::PressureMaterial, velocity_material::VelocityMaterial},
    plugins::{
        divergence::DivergencePlugin, mantle_grid::MantleGridPlugin, pressure::PressurePlugin,
        swappable_bind_group::SwappableBindGroupPlugin, velocity::VelocityPlugin,
        vertex_pressure::VertexPressurePlugin, vertex_velocity::VertexVelocityPlugin,
    },
    resources::{active_material::ActiveMaterial, gizmo_visibility::GizmoVisibility},
    systems::{
        gizmos::{
            draw_triangle_grid, draw_triangle_grid_centers, draw_triangle_grid_neighbors,
            draw_velocity_arrows, draw_vertex_velocity_arrows,
        },
        globe_visibility::update_globe_visibility,
        input::{
            GizmoAction, MaterialAction, gizmo_input_map, material_input_map,
            toggle_active_material, toggle_gizmo_visibility,
        },
        setup::setup,
    },
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(InputManagerPlugin::<GizmoAction>::default())
        .add_plugins(InputManagerPlugin::<MaterialAction>::default())
        .add_plugins(PanOrbitCameraPlugin)
        .add_plugins(SwappableBindGroupPlugin)
        .add_plugins(MantleGridPlugin)
        .add_plugins(PressurePlugin)
        .add_plugins(VertexPressurePlugin)
        .add_plugins(VelocityPlugin)
        .add_plugins(DivergencePlugin)
        .add_plugins(VertexVelocityPlugin)
        .add_plugins(MaterialPlugin::<PressureMaterial>::default())
        .add_plugins(MaterialPlugin::<VelocityMaterial>::default())
        .init_resource::<GizmoVisibility>()
        .init_resource::<ActiveMaterial>()
        .init_resource::<ActionState<GizmoAction>>()
        .init_resource::<ActionState<MaterialAction>>()
        .insert_resource(gizmo_input_map())
        .insert_resource(material_input_map())
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                toggle_gizmo_visibility,
                toggle_active_material,
                update_globe_visibility,
                draw_triangle_grid,
                draw_triangle_grid_centers,
                draw_triangle_grid_neighbors,
                draw_velocity_arrows,
                draw_vertex_velocity_arrows,
            ),
        )
        .run();
}
