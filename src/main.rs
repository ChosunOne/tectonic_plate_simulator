use bevy::prelude::*;
use bevy_panorbit_camera::PanOrbitCameraPlugin;
use leafwing_input_manager::{plugin::InputManagerPlugin, prelude::ActionState};
use tectonic_plate_simulator::{
    materials::{
        divergence_material::DivergenceMaterial, pressure_material::PressureMaterial,
        velocity_material::VelocityMaterial,
    },
    plugins::{
        advection::AdvectionPlugin, departure_info::DepartureInfoPlugin,
        divergence::DivergencePlugin, mantle_grid::MantleGridPlugin, pressure::PressurePlugin,
        sim_params::SimParamsPlugin, swappable_bind_group::SwappableBindGroupPlugin, ui::UiPlugin,
        velocity::VelocityPlugin, vertex_divergence::VertexDivergencePlugin,
        vertex_pressure::VertexPressurePlugin, vertex_velocity::VertexVelocityPlugin,
        viscosity::ViscosityPlugin,
    },
    resources::{
        active_material::ActiveMaterial, gizmo_visibility::GizmoVisibility,
        selected_edge::SelectedEdge,
    },
    systems::{
        gizmos::{
            draw_departure_gizmo, draw_triangle_grid, draw_triangle_grid_centers,
            draw_triangle_grid_neighbors, draw_velocity_arrows, draw_vertex_velocity_arrows,
        },
        globe_visibility::update_globe_visibility,
        input::{
            GizmoAction, MaterialAction, SelectionAction, SimulationAction, gizmo_input_map,
            handle_selection_input, handle_simulation_input, material_input_map,
            selection_input_map, simulation_input_map, toggle_active_material,
            toggle_gizmo_visibility,
        },
        setup::setup,
    },
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(InputManagerPlugin::<GizmoAction>::default())
        .add_plugins(InputManagerPlugin::<MaterialAction>::default())
        .add_plugins(InputManagerPlugin::<SimulationAction>::default())
        .add_plugins(InputManagerPlugin::<SelectionAction>::default())
        .add_plugins(PanOrbitCameraPlugin)
        .add_plugins(SwappableBindGroupPlugin)
        .add_plugins(SimParamsPlugin)
        .add_plugins(MantleGridPlugin)
        .add_plugins(PressurePlugin)
        .add_plugins(VertexPressurePlugin)
        .add_plugins(VelocityPlugin)
        .add_plugins(ViscosityPlugin)
        .add_plugins(AdvectionPlugin)
        .add_plugins(DepartureInfoPlugin)
        .add_plugins(DivergencePlugin)
        .add_plugins(VertexVelocityPlugin)
        .add_plugins(VertexDivergencePlugin)
        .add_plugins(UiPlugin)
        .add_plugins(MaterialPlugin::<PressureMaterial>::default())
        .add_plugins(MaterialPlugin::<VelocityMaterial>::default())
        .add_plugins(MaterialPlugin::<DivergenceMaterial>::default())
        .init_resource::<GizmoVisibility>()
        .init_resource::<ActiveMaterial>()
        .init_resource::<SelectedEdge>()
        .init_resource::<ActionState<GizmoAction>>()
        .init_resource::<ActionState<MaterialAction>>()
        .init_resource::<ActionState<SimulationAction>>()
        .init_resource::<ActionState<SelectionAction>>()
        .insert_resource(gizmo_input_map())
        .insert_resource(material_input_map())
        .insert_resource(simulation_input_map())
        .insert_resource(selection_input_map())
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                toggle_gizmo_visibility,
                toggle_active_material,
                handle_simulation_input,
                handle_selection_input,
                update_globe_visibility,
                draw_triangle_grid,
                draw_triangle_grid_centers,
                draw_triangle_grid_neighbors,
                draw_velocity_arrows,
                draw_vertex_velocity_arrows,
                draw_departure_gizmo,
            ),
        )
        .run();
}
