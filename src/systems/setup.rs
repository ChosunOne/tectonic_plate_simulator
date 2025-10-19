use bevy::{core_pipeline::Skybox, prelude::*};
use bevy_panorbit_camera::PanOrbitCamera;

use crate::resources::mantle_grid::MantleGrid;

pub fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // Spawn the sphere
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    let grid = MantleGrid::new(20);
    commands.insert_resource(grid);

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
