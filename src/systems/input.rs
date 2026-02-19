use bevy::{prelude::*, window::PrimaryWindow};
use leafwing_input_manager::prelude::*;

use crate::{
    constants::SPHERE_RADIUS,
    resources::{
        active_material::ActiveMaterial, gizmo_visibility::GizmoVisibility, mesh_grid::MeshGrid,
        selected_edge::SelectedEdge, simulation_time::SimulationTime,
    },
};

#[derive(Actionlike, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum GizmoAction {
    ToggleTriangleGrid,
    ToggleTriangleCenters,
    ToggleTriangleNeighbors,
    ToggleVelocityArrows,
    ToggleVertexVelocityArrows,
}

#[derive(Actionlike, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum MaterialAction {
    ShowPressure,
    ShowVelocity,
    ShowDivergence,
}

#[derive(Actionlike, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum SimulationAction {
    TogglePause,
    StepForward,
    SpeedUp,
    SlowDown,
}

#[derive(Actionlike, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum SelectionAction {
    Select,
}

pub fn toggle_gizmo_visibility(
    action_state: Res<ActionState<GizmoAction>>,
    mut visibility: ResMut<GizmoVisibility>,
) {
    let pressed = action_state.get_just_pressed();
    for action in pressed {
        match action {
            GizmoAction::ToggleTriangleGrid => visibility.triangle_grid = !visibility.triangle_grid,
            GizmoAction::ToggleTriangleCenters => {
                visibility.triangle_centers = !visibility.triangle_centers;
            }
            GizmoAction::ToggleTriangleNeighbors => {
                visibility.triangle_neighbors = !visibility.triangle_neighbors;
            }
            GizmoAction::ToggleVelocityArrows => {
                visibility.velocity_arrows = !visibility.velocity_arrows;
            }
            GizmoAction::ToggleVertexVelocityArrows => {
                visibility.vertex_velocity_arrows = !visibility.vertex_velocity_arrows;
            }
        }
    }
}

#[must_use]
pub fn gizmo_input_map() -> InputMap<GizmoAction> {
    InputMap::new([
        (
            GizmoAction::ToggleTriangleGrid,
            ButtonlikeChord::modified(ModifierKey::Control, KeyCode::Digit1),
        ),
        (
            GizmoAction::ToggleTriangleCenters,
            ButtonlikeChord::modified(ModifierKey::Control, KeyCode::Digit2),
        ),
        (
            GizmoAction::ToggleTriangleNeighbors,
            ButtonlikeChord::modified(ModifierKey::Control, KeyCode::Digit3),
        ),
        (
            GizmoAction::ToggleVelocityArrows,
            ButtonlikeChord::modified(ModifierKey::Control, KeyCode::Digit4),
        ),
        (
            GizmoAction::ToggleVertexVelocityArrows,
            ButtonlikeChord::modified(ModifierKey::Control, KeyCode::Digit5),
        ),
    ])
}

#[must_use]
pub fn material_input_map() -> InputMap<MaterialAction> {
    InputMap::new([
        (MaterialAction::ShowPressure, KeyCode::KeyP),
        (MaterialAction::ShowVelocity, KeyCode::KeyV),
        (MaterialAction::ShowDivergence, KeyCode::KeyD),
    ])
}

pub fn toggle_active_material(
    action_state: Res<ActionState<MaterialAction>>,
    mut active_material: ResMut<ActiveMaterial>,
) {
    let pressed = action_state.get_just_pressed();
    for action in pressed {
        match action {
            MaterialAction::ShowPressure => *active_material = ActiveMaterial::Pressure,
            MaterialAction::ShowVelocity => *active_material = ActiveMaterial::Velocity,
            MaterialAction::ShowDivergence => *active_material = ActiveMaterial::Divergence,
        }
    }
}

#[must_use]
pub fn simulation_input_map() -> InputMap<SimulationAction> {
    InputMap::new([
        (
            SimulationAction::TogglePause,
            ButtonlikeChord::modified(ModifierKey::Control, KeyCode::KeyP),
        ),
        (
            SimulationAction::StepForward,
            ButtonlikeChord::modified(ModifierKey::Control, KeyCode::KeyF),
        ),
        (
            SimulationAction::SpeedUp,
            ButtonlikeChord::modified(ModifierKey::Control, KeyCode::Period),
        ),
        (
            SimulationAction::SlowDown,
            ButtonlikeChord::modified(ModifierKey::Control, KeyCode::Comma),
        ),
    ])
}

pub fn handle_simulation_input(
    action_state: Res<ActionState<SimulationAction>>,
    sim_time: Res<SimulationTime>,
) {
    for action in action_state.get_just_pressed() {
        match action {
            SimulationAction::TogglePause => sim_time.toggle_pause(),
            SimulationAction::StepForward => {
                sim_time.pause();
                sim_time.step();
            }
            SimulationAction::SpeedUp => sim_time.double_speed(),
            SimulationAction::SlowDown => sim_time.half_speed(),
        }
    }
}

#[must_use]
pub fn selection_input_map() -> InputMap<SelectionAction> {
    InputMap::new([(SelectionAction::Select, MouseButton::Left)])
}

pub fn handle_selection_input(
    action_state: Res<ActionState<SelectionAction>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    grid: Res<MeshGrid>,
    mut selected_edge: ResMut<SelectedEdge>,
) {
    if !action_state.just_pressed(&SelectionAction::Select) {
        return;
    }

    let Ok(window) = windows.single() else {
        return;
    };

    let Some(cursor_position) = window.cursor_position() else {
        return;
    };

    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };

    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
        return;
    };

    let o_dot_d = ray.origin.dot(ray.direction.as_vec3());
    let o_dot_o = ray.origin.dot(ray.origin);
    let r_squared = SPHERE_RADIUS * SPHERE_RADIUS;

    let discriminant = o_dot_d * o_dot_d - (o_dot_o - r_squared);

    if discriminant < 0.0 {
        selected_edge.0 = None;
        return;
    }

    let sqrt_discriminant = discriminant.sqrt();
    let t1 = -o_dot_d - sqrt_discriminant;
    let t2 = -o_dot_d + sqrt_discriminant;

    let t = if t1 > 0.0 {
        t1
    } else if t2 > 0.0 {
        t2
    } else {
        selected_edge.0 = None;
        return;
    };

    let hit_point = ray.origin + t * ray.direction.as_vec3();

    let points = grid.points();
    let edge_vertex_adjacency = grid.edge_vertex_adjacency();
    let num_edges = edge_vertex_adjacency.rows();

    let mut nearest_edge = None;
    let mut nearest_distance_sq = f32::MAX;

    for edge_idx in 0..num_edges {
        let edge_verts = edge_vertex_adjacency
            .outer_view(edge_idx)
            .expect("to have vertices for edge")
            .iter()
            .map(|(_, &x)| x as usize)
            .collect::<Vec<_>>();
        let v_lower_pos = points[edge_verts[0]];
        let v_higher_pos = points[edge_verts[1]];

        let midpoint: Vec3 = ((v_lower_pos + v_higher_pos) / 2.0).into();
        let distance_sq = (hit_point - midpoint).length_squared();

        if distance_sq < nearest_distance_sq {
            nearest_distance_sq = distance_sq;
            nearest_edge = Some(edge_idx);
        }
    }

    selected_edge.0 = nearest_edge;
}
