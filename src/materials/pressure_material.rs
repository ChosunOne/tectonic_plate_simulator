use bevy::{
    prelude::*,
    render::{render_resource::AsBindGroup, storage::ShaderStorageBuffer},
    shader::ShaderRef,
};

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct PressureMaterial {
    #[storage(0, read_only, visibility(vertex))]
    pub vertex_pressure: Handle<ShaderStorageBuffer>,
}

impl Material for PressureMaterial {
    fn vertex_shader() -> ShaderRef {
        "shaders/pressure_material.wgsl".into()
    }
    fn fragment_shader() -> ShaderRef {
        "shaders/pressure_material.wgsl".into()
    }
}
