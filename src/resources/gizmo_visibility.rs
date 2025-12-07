use bevy::ecs::resource::Resource;

#[derive(Resource, Default)]
pub struct GizmoVisibility {
    pub triangle_grid: bool,
    pub triangle_centers: bool,
    pub triangle_neighbors: bool,
}
