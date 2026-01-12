use bevy::ecs::resource::Resource;

#[allow(clippy::struct_excessive_bools)]
#[derive(Resource)]
pub struct GizmoVisibility {
    pub triangle_grid: bool,
    pub triangle_centers: bool,
    pub triangle_neighbors: bool,
    pub velocity_arrows: bool,
    pub vertex_velocity_arrows: bool,
}

impl Default for GizmoVisibility {
    fn default() -> Self {
        Self {
            vertex_velocity_arrows: false,
            velocity_arrows: false,
            triangle_grid: false,
            triangle_centers: false,
            triangle_neighbors: false,
        }
    }
}
