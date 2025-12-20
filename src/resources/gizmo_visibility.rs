use bevy::ecs::resource::Resource;

#[allow(clippy::struct_excessive_bools)]
#[derive(Resource, Default)]
pub struct GizmoVisibility {
    pub triangle_grid: bool,
    pub triangle_centers: bool,
    pub triangle_neighbors: bool,
    pub velocity_arrows: bool,
}
