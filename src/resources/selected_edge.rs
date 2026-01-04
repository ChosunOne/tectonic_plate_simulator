use bevy::ecs::resource::Resource;

#[derive(Resource, Default, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SelectedEdge(pub Option<usize>);
