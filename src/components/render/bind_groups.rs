use bevy::ecs::component::Component;

/// Marker component labeling a `SwappableBindGroup` as a `PressureBindGroup`.
#[derive(Component)]
pub struct PressureBindGroup;

/// Marker component labeling a `SwappableBindGroup` as a `VelocityBindGroup`.
#[derive(Component)]
pub struct VelocityBindGroup;
