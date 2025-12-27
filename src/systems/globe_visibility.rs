use bevy::{
    camera::visibility::Visibility,
    ecs::{
        change_detection::DetectChanges,
        query::{With, Without},
        system::{Query, Res},
    },
};

use crate::{
    components::globe::{DivergenceGlobe, PressureGlobe, VelocityGlobe},
    resources::active_material::ActiveMaterial,
};

pub fn update_globe_visibility(
    active_material: Res<ActiveMaterial>,
    mut pressure_query: Query<
        &mut Visibility,
        (
            With<PressureGlobe>,
            Without<VelocityGlobe>,
            Without<DivergenceGlobe>,
        ),
    >,
    mut velocity_query: Query<
        &mut Visibility,
        (
            With<VelocityGlobe>,
            Without<PressureGlobe>,
            Without<DivergenceGlobe>,
        ),
    >,
    mut divergence_query: Query<
        &mut Visibility,
        (
            With<DivergenceGlobe>,
            Without<VelocityGlobe>,
            Without<PressureGlobe>,
        ),
    >,
) {
    if !active_material.is_changed() || active_material.is_added() {
        return;
    }

    for mut visibility in &mut pressure_query {
        *visibility = match *active_material {
            ActiveMaterial::Pressure => Visibility::Visible,
            _ => Visibility::Hidden,
        }
    }

    for mut visibility in &mut velocity_query {
        *visibility = match *active_material {
            ActiveMaterial::Velocity => Visibility::Visible,
            _ => Visibility::Hidden,
        }
    }

    for mut visibility in &mut divergence_query {
        *visibility = match *active_material {
            ActiveMaterial::Divergence => Visibility::Visible,
            _ => Visibility::Hidden,
        }
    }
}
