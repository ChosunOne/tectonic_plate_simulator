use bevy::{core_pipeline::Skybox, prelude::*};
use bevy_panorbit_camera::PanOrbitCamera;

use crate::{
    components::globe::{DivergenceGlobe, PressureGlobe, VelocityGlobe},
    constants::SPHERE_RADIUS,
    materials::{
        divergence_material::DivergenceMaterial, pressure_material::PressureMaterial,
        velocity_material::VelocityMaterial,
    },
    resources::mantle_grid::MantleGrid,
};

pub fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut pressure_materials: ResMut<Assets<PressureMaterial>>,
    mut velocity_materials: ResMut<Assets<VelocityMaterial>>,
    mut divergence_materials: ResMut<Assets<DivergenceMaterial>>,
    asset_server: Res<AssetServer>,
    grid: Res<MantleGrid>,
) {
    let mesh = grid.mesh();
    let mesh_handle = meshes.add(mesh);

    commands.spawn((
        Mesh3d(mesh_handle.clone()),
        MeshMaterial3d(pressure_materials.add(PressureMaterial)),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Visibility::Visible,
        PressureGlobe,
    ));

    commands.spawn((
        Mesh3d(mesh_handle.clone()),
        MeshMaterial3d(velocity_materials.add(VelocityMaterial)),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Visibility::Hidden,
        VelocityGlobe,
    ));

    commands.spawn((
        Mesh3d(mesh_handle),
        MeshMaterial3d(divergence_materials.add(DivergenceMaterial)),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Visibility::Hidden,
        DivergenceGlobe,
    ));

    // Spawn the camera
    commands.spawn((
        PanOrbitCamera {
            zoom_upper_limit: Some(10.0 * SPHERE_RADIUS),
            zoom_lower_limit: 2.0 * SPHERE_RADIUS,
            pan_sensitivity: 0.0,
            radius: Some(SPHERE_RADIUS * 5.0),
            ..Default::default()
        },
        Transform::from_xyz(0.0, SPHERE_RADIUS / 5.0, 5.0 * SPHERE_RADIUS)
            .looking_at(Vec3::ZERO, Vec3::Y),
        Skybox {
            image: asset_server.load("textures/Standard-Cube-Map/stars.ktx2"),

            brightness: 150.0,
            ..Default::default()
        },
    ));
}
