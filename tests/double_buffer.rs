use std::time::Duration;

use bevy::{
    DefaultPlugins,
    app::{App, Plugin, ScheduleRunnerPlugin},
    asset::AssetServer,
    ecs::{
        resource::Resource,
        schedule::IntoScheduleConfigs,
        system::{Commands, Res, ResMut},
        world::World,
    },
    prelude::PluginGroup,
    render::{
        Render, RenderApp, RenderPlugin, RenderStartup, RenderSystems,
        render_graph::{Node, NodeRunError, RenderGraph, RenderGraphContext, RenderLabel},
        render_resource::{
            CachedComputePipelineId, ComputePassDescriptor, ComputePipelineDescriptor,
            PipelineCache,
        },
        renderer::{RenderContext, RenderDevice, RenderQueue},
    },
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use tectonic_plate_simulator::{
    components::render::swappable_bind_group::{BindGroupBuilder, SwappableBindGroup},
    plugins::swappable_bind_group::{SwappableBindGroupPlugin, swap_bind_groups},
    render::double_buffer::DoubleBuffer,
};

const BUFFER_SIZE: usize = 256;

#[derive(Resource)]
struct IncrementPipeline {
    pipeline_id: CachedComputePipelineId,
}

#[derive(Resource)]
struct TestDoubleBuffer(DoubleBuffer<u32>);

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct IncrementLabel;

struct IncrementNode;

impl Node for IncrementNode {
    #[allow(clippy::cast_possible_truncation)]
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let increment_pipeline = world.resource::<IncrementPipeline>();
        let Some(mut query) = world.try_query::<&SwappableBindGroup>() else {
            return Ok(());
        };
        let bind_group = query.single(world).expect("To have bind group");

        let Some(pipeline) = pipeline_cache.get_compute_pipeline(increment_pipeline.pipeline_id)
        else {
            return Ok(());
        };

        let mut pass =
            render_context
                .command_encoder()
                .begin_compute_pass(&ComputePassDescriptor {
                    label: Some("increment_pass"),
                    timestamp_writes: None,
                });

        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, bind_group.current(), &[]);
        pass.dispatch_workgroups((BUFFER_SIZE as u32).div_ceil(64), 1, 1);

        Ok(())
    }
}

struct ComputeTestPlugin;

impl Plugin for ComputeTestPlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(RenderStartup, add_render_graph_node);
        render_app.add_systems(RenderStartup, setup_render_resources);
    }
}

fn add_render_graph_node(mut render_graph: ResMut<RenderGraph>) {
    render_graph.add_node(IncrementLabel, IncrementNode);
}

struct ComputeTestResultsPlugin;

impl Plugin for ComputeTestResultsPlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(
            Render,
            check_compute_results
                .in_set(RenderSystems::Cleanup)
                .after(swap_bind_groups),
        );
    }
}

fn check_compute_results(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    test_double_buffer: Res<TestDoubleBuffer>,
) {
    let read_data = test_double_buffer
        .0
        .read_back_read_buffer(&render_device, &render_queue);
    let write_data = test_double_buffer
        .0
        .read_back_write_buffer(&render_device, &render_queue);
    if read_data[0] < write_data[0] {
        return;
    }
    for (&a, &b) in read_data.iter().zip(write_data.iter()) {
        assert_eq!(a, b + 1);
    }
}

fn setup_render_resources(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    mut pipeline_cache: ResMut<PipelineCache>,
    asset_server: Res<AssetServer>,
) {
    let data = vec![1u32; BUFFER_SIZE];
    let double_buffer = DoubleBuffer::new(&render_device, &data, Some("test_buffer"));

    commands.insert_resource(TestDoubleBuffer(double_buffer.clone()));
    let mut builder = BindGroupBuilder::new();
    builder.add_compute_double(double_buffer);
    let swappable = builder.build(&render_device, Some("test_bind_group"));
    let shader = asset_server.load("shaders/tests/double_buffer.wgsl");
    let pipeline_id = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("increment_pipeline".into()),
        layout: vec![swappable.layout().clone()],
        shader,
        shader_defs: vec![],
        entry_point: Some("main".into()),
        push_constant_ranges: vec![],
        zero_initialize_workgroup_memory: true,
    });

    commands.insert_resource(IncrementPipeline { pipeline_id });
    commands.spawn(swappable);
}

#[test]
fn test_double_buffer_increment() {
    let mut app = App::new();

    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                close_when_requested: false,
                ..Default::default()
            })
            .set(RenderPlugin {
                synchronous_pipeline_compilation: true,
                ..Default::default()
            })
            .disable::<WinitPlugin>(),
        ScheduleRunnerPlugin::run_loop(Duration::from_millis(16)),
        SwappableBindGroupPlugin,
        ComputeTestPlugin,
        ComputeTestResultsPlugin,
    ));

    app.finish();
    app.cleanup();

    let num_frames = 10;
    for _ in 0..num_frames {
        app.update();
    }
}
