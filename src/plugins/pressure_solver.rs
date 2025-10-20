use bevy::{
    prelude::*,
    render::{
        Render, RenderApp, RenderSystems,
        extract_resource::ExtractResourcePlugin,
        render_asset::RenderAssets,
        render_resource::{
            BindGroupEntry, BindGroupLayout, BindGroupLayoutEntry, BindingType, BufferBindingType,
            BufferDescriptor, BufferUsages, CachedComputePipelineId, ComputePipelineDescriptor,
            MapMode, PipelineCache, PollType, ShaderStages,
        },
        renderer::{RenderDevice, RenderQueue},
        storage::GpuShaderStorageBuffer,
    },
};

use crate::resources::{
    mantle_grid::MantleGrid,
    pressure_buffers::{PressureBuffers, prepare_buffers},
    vertex_pressure_buffer::VertexPressureBufferHandle,
};

pub struct PressureSolverPlugin;

impl Plugin for PressureSolverPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractResourcePlugin::<MantleGrid>::default());

        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(
            Render,
            (
                prepare_pipeline,
                prepare_vertex_pressure_pipeline,
                prepare_buffers,
                dispatch_pressure_solver,
                dispatch_vertex_pressure_solver,
                readback_pressure,
            )
                .chain()
                .in_set(RenderSystems::Prepare),
        );
    }
}

#[derive(Resource)]
pub struct PressureSolverPipeline {
    pub bind_group_layout: BindGroupLayout,
    pub pipeline_id: CachedComputePipelineId,
}

fn prepare_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    pipeline_cache: Res<PipelineCache>,
    asset_server: Res<AssetServer>,
    pipeline: Option<Res<PressureSolverPipeline>>,
) {
    if pipeline.is_some() {
        return;
    }
    let shader = asset_server.load("shaders/pressure_solver.wgsl");
    let bind_group_layout = render_device.create_bind_group_layout(
        "pressure_solver_bind_group_layout",
        &[
            // pressure_in
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // pressure_out
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // neighbors
            BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    );

    let pipeline_id = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("pressure_solver_pipeline".into()),
        layout: vec![bind_group_layout.clone()],
        shader,
        shader_defs: vec![],
        entry_point: Some("main".into()),
        push_constant_ranges: vec![],
        zero_initialize_workgroup_memory: true,
    });

    commands.insert_resource(PressureSolverPipeline {
        bind_group_layout,
        pipeline_id,
    });
}

pub fn dispatch_pressure_solver(
    pipeline: Res<PressureSolverPipeline>,
    mut buffers: ResMut<PressureBuffers>,
    pipeline_cache: Res<PipelineCache>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    let Some(compute_pipeline) = pipeline_cache.get_compute_pipeline(pipeline.pipeline_id) else {
        return;
    };

    // Ping-pong: read from one, write to other
    let (read_buffer, write_buffer) = if buffers.current_read {
        (&buffers.pressure_buffer_a, &buffers.pressure_buffer_b)
    } else {
        (&buffers.pressure_buffer_b, &buffers.pressure_buffer_a)
    };

    let bind_group = render_device.create_bind_group(
        "pressure_solver_bind_group",
        &pipeline.bind_group_layout,
        &[
            BindGroupEntry {
                binding: 0,
                resource: read_buffer.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: write_buffer.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 2,
                resource: buffers.neighbors_buffer.as_entire_binding(),
            },
        ],
    );

    let mut encoder = render_device.create_command_encoder(&Default::default());
    {
        let mut compute_pass = encoder.begin_compute_pass(&Default::default());
        compute_pass.set_pipeline(compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);
        let workgroups = buffers.num_cells.div_ceil(64);
        compute_pass.dispatch_workgroups(workgroups, 1, 1);
    }

    render_queue.submit(std::iter::once(encoder.finish()));
    buffers.current_read = !buffers.current_read;
}

#[derive(Resource)]
pub struct VertexPressurePipeline {
    pub bind_group_layout: BindGroupLayout,
    pub pipeline_id: CachedComputePipelineId,
}

fn prepare_vertex_pressure_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    pipeline_cache: Res<PipelineCache>,
    asset_server: Res<AssetServer>,
    pipeline: Option<Res<VertexPressurePipeline>>,
) {
    if pipeline.is_some() {
        return;
    }

    let shader = asset_server.load("shaders/vertex_pressure.wgsl");
    let bind_group_layout = render_device.create_bind_group_layout(
        "vertex_pressure_bind_group_layout",
        &[
            // triangle pressure
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // vertex pressure
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // vertex triangles
            BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    );

    let pipeline_id = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("vertex_pressure_pipeline".into()),
        layout: vec![bind_group_layout.clone()],
        push_constant_ranges: vec![],
        shader,
        shader_defs: vec![],
        entry_point: Some("main".into()),
        zero_initialize_workgroup_memory: true,
    });

    commands.insert_resource(VertexPressurePipeline {
        bind_group_layout,
        pipeline_id,
    });
}

fn dispatch_vertex_pressure_solver(
    pipeline: Res<VertexPressurePipeline>,
    buffers: Res<PressureBuffers>,
    vertex_buffer_handle: Res<VertexPressureBufferHandle>,
    gpu_buffers: Res<RenderAssets<GpuShaderStorageBuffer>>,
    pipeline_cache: Res<PipelineCache>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    let Some(compute_pipeline) = pipeline_cache.get_compute_pipeline(pipeline.pipeline_id) else {
        return;
    };

    let Some(vertex_gpu_buffer) = gpu_buffers.get(&vertex_buffer_handle.0) else {
        return;
    };

    let read_buffer = if buffers.current_read {
        &buffers.pressure_buffer_b
    } else {
        &buffers.pressure_buffer_a
    };

    let bind_group = render_device.create_bind_group(
        "vertex_pressure_bind_group",
        &pipeline.bind_group_layout,
        &[
            BindGroupEntry {
                binding: 0,
                resource: read_buffer.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: vertex_gpu_buffer.buffer.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 2,
                resource: buffers.vertex_triangles_buffer.as_entire_binding(),
            },
        ],
    );

    let mut encoder = render_device.create_command_encoder(&Default::default());
    {
        let mut compute_pass = encoder.begin_compute_pass(&Default::default());
        compute_pass.set_pipeline(compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);
        let workgroups = buffers.num_vertices.div_ceil(64);
        compute_pass.dispatch_workgroups(workgroups, 1, 1);
    }

    render_queue.submit(std::iter::once(encoder.finish()));
}

fn readback_pressure(
    buffers: Res<PressureBuffers>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    static mut COUNTER: u32 = 0;
    unsafe {
        COUNTER += 1;
        if COUNTER % 60 != 0 {
            return;
        }
    }

    let staging_buffer = render_device.create_buffer(&BufferDescriptor {
        label: Some("pressure_readback"),
        size: u64::from(buffers.num_cells) * 4,
        usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let read_buffer = if buffers.current_read {
        &buffers.pressure_buffer_b
    } else {
        &buffers.pressure_buffer_a
    };

    let mut encoder = render_device.create_command_encoder(&Default::default());
    encoder.copy_buffer_to_buffer(
        read_buffer,
        0,
        &staging_buffer,
        0,
        u64::from(buffers.num_cells) * 4,
    );
    render_queue.submit(std::iter::once(encoder.finish()));
    let buffer_slice = staging_buffer.slice(..);
    buffer_slice.map_async(MapMode::Read, |_| {});
    render_device.poll(PollType::Wait).expect("Failed to wait");

    let data = buffer_slice.get_mapped_range();
    let pressures: &[f32] = bytemuck::cast_slice(&data);

    println!("{:?}", &pressures[..10]);

    drop(data);
    staging_buffer.unmap();
}
