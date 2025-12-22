use bevy::{
    camera::visibility::Visibility,
    ecs::{
        change_detection::DetectChanges,
        query::{With, Without},
        system::{Query, Res},
    },
};

use crate::{
    components::globe::{PressureGlobe, VelocityGlobe},
    resources::active_material::ActiveMaterial,
};

pub fn update_globe_visibility(
    active_material: Res<ActiveMaterial>,
    mut pressure_query: Query<&mut Visibility, (With<PressureGlobe>, Without<VelocityGlobe>)>,
    mut velocity_query: Query<&mut Visibility, (With<VelocityGlobe>, Without<PressureGlobe>)>,
) {
    if !active_material.is_changed() || active_material.is_added() {
        return;
    }

    for mut visibility in &mut pressure_query {
        *visibility = match *active_material {
            ActiveMaterial::Pressure => Visibility::Visible,
            ActiveMaterial::Velocity => Visibility::Hidden,
        }
    }

    for mut visibility in &mut velocity_query {
        *visibility = match *active_material {
            ActiveMaterial::Pressure => Visibility::Hidden,
            ActiveMaterial::Velocity => Visibility::Visible,
        }
    }
}
