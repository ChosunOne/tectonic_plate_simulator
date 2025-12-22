use bevy::ecs::resource::Resource;

#[derive(Resource, Default, Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum ActiveMaterial {
    #[default]
    Pressure,
    Velocity,
}
