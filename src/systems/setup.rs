use bevy::{core_pipeline::Skybox, prelude::*};
use bevy_panorbit_camera::PanOrbitCamera;

use crate::{materials::pressure_material::PressureMaterial, resources::mantle_grid::MantleGrid};

pub fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    mut pressure_materials: ResMut<Assets<PressureMaterial>>,
    asset_server: Res<AssetServer>,
    grid: Res<MantleGrid>,
) {
    // Spawn the sphere
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.8))),
        MeshMaterial3d(standard_materials.add(Color::srgb(0.8, 0.7, 0.6))),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    let mesh = grid.mesh();

    commands.spawn((
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(pressure_materials.add(PressureMaterial)),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    // Spawn the camera
    commands.spawn((
        PanOrbitCamera {
            zoom_upper_limit: Some(10.0),
            zoom_lower_limit: 2.0,
            pan_sensitivity: 0.0,
            ..Default::default()
        },
        Transform::from_xyz(0.0, 2.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        Skybox {
            image: asset_server.load("textures/Standard-Cube-Map/stars.ktx2"),

            brightness: 150.0,
            ..Default::default()
        },
    ));

    // Add a light
    commands.spawn((
        PointLight {
            intensity: 1500.0,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
}
