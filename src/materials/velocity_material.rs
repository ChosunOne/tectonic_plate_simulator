use bevy::{
    asset::Asset,
    ecs::{
        query::With,
        system::{SystemParamItem, lifetimeless::SQuery},
    },
    pbr::Material,
    reflect::TypePath,
    render::{
        render_resource::{
            AsBindGroup, AsBindGroupError, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResources, BindingType,
            BufferBindingType, PipelineCache, PreparedBindGroup, ShaderStages, UnpreparedBindGroup,
        },
        renderer::RenderDevice,
    },
    shader::ShaderRef,
};

use crate::components::render::{
    SwappableBindGroup, TopologyBindGroup, VertexVelocityBindGroup,
    VertexVelocityReductionBindGroup,
};

#[derive(Asset, TypePath, Debug, Clone, Default, Copy, PartialEq, Eq, Hash)]
pub struct VelocityMaterial;

type VertexVelocityQuery = SQuery<&'static SwappableBindGroup, With<VertexVelocityBindGroup>>;

type VelocityBoundsQuery =
    SQuery<&'static SwappableBindGroup, With<VertexVelocityReductionBindGroup>>;

type TopologyQuery = SQuery<&'static SwappableBindGroup, With<TopologyBindGroup>>;

impl AsBindGroup for VelocityMaterial {
    type Data = ();
    type Param = (VertexVelocityQuery, VelocityBoundsQuery, TopologyQuery);

    fn label() -> &'static str {
        "velocity_material"
    }

    fn as_bind_group(
        &self,
        layout_descriptor: &BindGroupLayoutDescriptor,
        render_device: &RenderDevice,
        cache: &PipelineCache,
        (vertex_velocity_query, velocity_bounds_query, topology_query): &mut SystemParamItem<
            '_,
            '_,
            Self::Param,
        >,
    ) -> Result<PreparedBindGroup, AsBindGroupError> {
        let vertex_velocity_bind_group = vertex_velocity_query
            .single()
            .map_err(|_| AsBindGroupError::RetryNextUpdate)?;

        let vertex_velocity_buffer = vertex_velocity_bind_group
            .get_buffer(0)
            .ok_or(AsBindGroupError::RetryNextUpdate)?;

        let velocity_bounds_bind_group = velocity_bounds_query
            .single()
            .map_err(|_| AsBindGroupError::RetryNextUpdate)?;

        let velocity_bounds_buffer = velocity_bounds_bind_group
            .get_buffer(1)
            .ok_or(AsBindGroupError::RetryNextUpdate)?;

        let topology_bind_group = topology_query
            .single()
            .map_err(|_| AsBindGroupError::RetryNextUpdate)?;

        let vertex_angle_offsets_buffer = topology_bind_group
            .get_buffer(6)
            .ok_or(AsBindGroupError::RetryNextUpdate)?;

        let layout = cache.get_bind_group_layout(layout_descriptor);

        let bind_group = render_device.create_bind_group(
            Some("velocity_material_bind_group"),
            &layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: vertex_velocity_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: velocity_bounds_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: vertex_angle_offsets_buffer.as_entire_binding(),
                },
            ],
        );

        Ok(PreparedBindGroup {
            bindings: BindingResources(vec![]),
            bind_group,
        })
    }

    fn bind_group_data(&self) -> Self::Data {}

    fn unprepared_bind_group(
        &self,
        _layout: &BindGroupLayout,
        _render_device: &RenderDevice,
        _param: &mut SystemParamItem<'_, '_, Self::Param>,
        _force_no_bindless: bool,
    ) -> Result<UnpreparedBindGroup, AsBindGroupError> {
        Err(AsBindGroupError::CreateBindGroupDirectly)
    }

    fn bind_group_layout_entries(
        _render_device: &RenderDevice,
        _force_no_bindless: bool,
    ) -> Vec<BindGroupLayoutEntry>
    where
        Self: Sized,
    {
        vec![
            // @group(#{MATERIAL_BIND_GROUP}) @binding(0) var<storage, read>
            // vertex_velocity: array<vec2<f32>>;
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // @group(#{MATERIAL_BIND_GROUP}) @binding(1) var<storage, read>
            // vertex_velocity_bounds: array<f32, 2>;
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // @group(#{MATERIAL_BIND_GROUP}) @binding(2) var<storage, read>
            // vertex_angle_offsets: array<f32>;
            BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ]
    }
}

impl Material for VelocityMaterial {
    fn vertex_shader() -> ShaderRef {
        "shaders/velocity_material.wgsl".into()
    }

    fn fragment_shader() -> ShaderRef {
        "shaders/velocity_material.wgsl".into()
    }
}
