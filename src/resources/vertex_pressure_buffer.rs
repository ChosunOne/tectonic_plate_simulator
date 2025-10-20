use bevy::prelude::*;
use bevy::render::extract_resource::ExtractResource;
use bevy::render::storage::ShaderStorageBuffer;

#[derive(Resource, ExtractResource, Clone)]
pub struct VertexPressureBufferHandle(pub Handle<ShaderStorageBuffer>);
