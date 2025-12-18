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
            AsBindGroup, AsBindGroupError, BindGroupEntry, BindGroupLayout, BindGroupLayoutEntry,
            BindingResources, BindingType, BufferBindingType, PreparedBindGroup, ShaderStages,
            UnpreparedBindGroup,
        },
        renderer::RenderDevice,
    },
    shader::ShaderRef,
};

use crate::components::render::{
    SwappableBindGroup, VertexPressureBindGroup, VertexPressureReductionBindGroup,
};

#[derive(Asset, TypePath, Debug, Clone, Default)]
pub struct PressureMaterial;

type VertexPressureQuery = SQuery<&'static SwappableBindGroup, With<VertexPressureBindGroup>>;

type PressureBoundsQuery =
    SQuery<&'static SwappableBindGroup, With<VertexPressureReductionBindGroup>>;

impl AsBindGroup for PressureMaterial {
    type Data = ();
    type Param = (VertexPressureQuery, PressureBoundsQuery);

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
            // vertex_pressure: array<f32>;
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
            // vertex_pressure_bounds: array<f32, 2>;
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
        ]
    }

    fn as_bind_group(
        &self,
        layout: &BindGroupLayout,
        render_device: &RenderDevice,
        (vertex_pressure_query, vertex_pressure_bounds_query): &mut SystemParamItem<
            '_,
            '_,
            Self::Param,
        >,
    ) -> Result<PreparedBindGroup, AsBindGroupError> {
        let vertex_pressure_bind_group = vertex_pressure_query
            .single()
            .map_err(|_| AsBindGroupError::RetryNextUpdate)?;

        let vertex_pressure_buffer = vertex_pressure_bind_group
            .get_buffer(2)
            .ok_or(AsBindGroupError::RetryNextUpdate)?;

        let vertex_pressure_bounds_bind_group = vertex_pressure_bounds_query
            .single()
            .map_err(|_| AsBindGroupError::RetryNextUpdate)?;

        let vertex_pressure_bounds_buffer = vertex_pressure_bounds_bind_group
            .get_buffer(1)
            .ok_or(AsBindGroupError::RetryNextUpdate)?;

        let bind_group = render_device.create_bind_group(
            Some("pressure_material_bind_group"),
            layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: vertex_pressure_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: vertex_pressure_bounds_buffer.as_entire_binding(),
                },
            ],
        );

        Ok(PreparedBindGroup {
            bindings: BindingResources(vec![]),
            bind_group,
        })
    }
}

impl Material for PressureMaterial {
    fn vertex_shader() -> ShaderRef {
        "shaders/pressure_material.wgsl".into()
    }

    fn fragment_shader() -> ShaderRef {
        "shaders/pressure_material.wgsl".into()
    }
}
