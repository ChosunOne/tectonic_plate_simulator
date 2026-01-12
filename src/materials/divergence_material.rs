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
    SwappableBindGroup, VertexDivergenceBindGroup, VertexDivergenceReductionBindGroup,
};

#[derive(Asset, TypePath, Debug, Clone, Default)]
pub struct DivergenceMaterial;

type VertexDivergenceQuery = SQuery<&'static SwappableBindGroup, With<VertexDivergenceBindGroup>>;

type DivergenceBoundsQuery =
    SQuery<&'static SwappableBindGroup, With<VertexDivergenceReductionBindGroup>>;

impl AsBindGroup for DivergenceMaterial {
    type Data = ();
    type Param = (VertexDivergenceQuery, DivergenceBoundsQuery);

    fn label() -> &'static str {
        "divergence_material"
    }

    fn as_bind_group(
        &self,
        layout_descriptor: &BindGroupLayoutDescriptor,
        render_device: &RenderDevice,
        cache: &PipelineCache,
        (vertex_divergence_query, vertex_divergence_bounds_query): &mut SystemParamItem<
            '_,
            '_,
            Self::Param,
        >,
    ) -> Result<PreparedBindGroup, AsBindGroupError> {
        let vertex_divergence_bind_group = vertex_divergence_query
            .single()
            .map_err(|_| AsBindGroupError::RetryNextUpdate)?;

        let vertex_divergence_buffer = vertex_divergence_bind_group
            .get_buffer(0)
            .ok_or(AsBindGroupError::RetryNextUpdate)?;

        let vertex_divergence_bounds_bind_group = vertex_divergence_bounds_query
            .single()
            .map_err(|_| AsBindGroupError::RetryNextUpdate)?;

        let vertex_divergence_bounds_buffer = vertex_divergence_bounds_bind_group
            .get_buffer(1)
            .ok_or(AsBindGroupError::RetryNextUpdate)?;

        let layout = cache.get_bind_group_layout(layout_descriptor);

        let bind_group = render_device.create_bind_group(
            Some("divergence_material_bind_group"),
            &layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: vertex_divergence_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: vertex_divergence_bounds_buffer.as_entire_binding(),
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
            // vertex_divergence: array<f32>;
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
            // vertex_divergence_bounds: array<f32, 2>;
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
}

impl Material for DivergenceMaterial {
    fn vertex_shader() -> ShaderRef {
        "shaders/divergence_material.wgsl".into()
    }

    fn fragment_shader() -> ShaderRef {
        "shaders/divergence_material.wgsl".into()
    }
}
