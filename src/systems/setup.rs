use bevy::{core_pipeline::Skybox, prelude::*, render::storage::ShaderStorageBuffer};
use bevy_panorbit_camera::PanOrbitCamera;

use crate::{
    materials::pressure_material::PressureMaterial,
    resources::{mantle_grid::MantleGrid, vertex_pressure_buffer::VertexPressureBufferHandle},
};

pub fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut pressure_materials: ResMut<Assets<PressureMaterial>>,
    mut storage_buffers: ResMut<Assets<ShaderStorageBuffer>>,
    asset_server: Res<AssetServer>,
) {
    // Spawn the sphere
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.8))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    let grid = MantleGrid::new(20);
    let mesh = grid.mesh();

    let num_vertices = grid.sphere.raw_points().len();
    let vertex_pressure_data = vec![0.0f32; num_vertices];
    let mut vertex_pressure_buffer_asset = ShaderStorageBuffer::from(vertex_pressure_data);
    vertex_pressure_buffer_asset.buffer_description.usage |=
        bevy::render::render_resource::BufferUsages::STORAGE;
    let vertex_pressure_buffer = storage_buffers.add(vertex_pressure_buffer_asset);
    commands.insert_resource(VertexPressureBufferHandle(vertex_pressure_buffer.clone()));

    commands.spawn((
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(pressure_materials.add(PressureMaterial {
            vertex_pressure: vertex_pressure_buffer,
        })),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
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
