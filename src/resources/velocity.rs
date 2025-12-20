use std::sync::{Arc, Mutex};

use bevy::ecs::resource::Resource;

/// Structure to share Velocity information between the render and the main world.
#[derive(Resource, Clone)]
pub struct VelocitySync(pub Arc<Mutex<Vec<[f32; 2]>>>);

impl Default for VelocitySync {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(Vec::new())))
    }
}
