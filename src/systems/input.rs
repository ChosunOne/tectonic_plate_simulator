use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

use crate::resources::{active_material::ActiveMaterial, gizmo_visibility::GizmoVisibility};

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
        }
    }
}
