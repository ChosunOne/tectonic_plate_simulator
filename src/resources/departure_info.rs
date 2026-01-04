use std::sync::{Arc, Mutex};

use bevy::ecs::resource::Resource;
use bytemuck::{Pod, Zeroable};

#[derive(Copy, Clone, Debug, Default, Pod, Zeroable)]
#[repr(C, align(16))]
pub struct DepartureInfo {
    pub base_edge: u32,
    pub cell: u32,
    pub pos: [f32; 2],
    pub interpolated_velocity: [f32; 2],
    pub last_velocity: [f32; 2],
}

#[derive(Resource, Clone, Default, Debug)]
pub struct DepartureInfoSync(pub Arc<Mutex<Vec<DepartureInfo>>>);
