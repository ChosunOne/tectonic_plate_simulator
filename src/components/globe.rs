use bevy::ecs::component::Component;

#[derive(Component, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PressureGlobe;

#[derive(Component, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct VelocityGlobe;

#[derive(Component, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct DivergenceGlobe;
