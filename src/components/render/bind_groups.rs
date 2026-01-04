use bevy::ecs::component::Component;

#[derive(Component, Copy, Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct PressureBindGroup;

#[derive(Component, Copy, Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct VelocityBindGroup;

#[derive(Component, Copy, Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct VertexPressureBindGroup;

#[derive(Component, Copy, Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct VertexPressureReductionBindGroup;

#[derive(Component, Copy, Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct TopologyBindGroup;

#[derive(Component, Copy, Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct VertexVelocityBindGroup;

#[derive(Component, Copy, Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct VertexVelocityReductionBindGroup;

#[derive(Component, Copy, Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct DivergenceBindGroup;

#[derive(Component, Copy, Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct VertexDivergenceBindGroup;

#[derive(Component, Copy, Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct VertexDivergenceReductionBindGroup;

#[derive(Component, Copy, Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct PhiBindGroup;

#[derive(Component, Copy, Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct SimParamsBindGroup;

#[derive(Component, Copy, Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct DepartureBindGroup;
