use std::sync::{Arc, Mutex};

use bevy::ecs::resource::Resource;

#[derive(Resource, Clone, Debug)]
pub struct VertexVelocitySync(pub Arc<Mutex<Vec<[f32; 2]>>>);

impl VertexVelocitySync {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for VertexVelocitySync {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(Vec::new())))
    }
}
