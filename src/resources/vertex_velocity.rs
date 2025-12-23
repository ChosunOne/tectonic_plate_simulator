use std::sync::{Arc, Mutex};

use bevy::ecs::resource::Resource;

#[derive(Resource, Clone, Debug)]
pub struct VertexVelocitySync(pub Arc<Mutex<Vec<[f32; 2]>>>);

impl Default for VertexVelocitySync {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(Vec::new())))
    }
}
