use std::time::Duration;

use bevy::{
    DefaultPlugins,
    app::{App, Plugin, ScheduleRunnerPlugin},
    asset::AssetServer,
    ecs::{
        component::Component,
        query::With,
        resource::Resource,
        schedule::IntoScheduleConfigs,
        system::{Commands, Query, Res, ResMut},
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
    components::render::{BindGroupBuilder, SwappableBindGroup},
    plugins::swappable_bind_group::{SwappableBindGroupPlugin, clear_step},
    render::double_buffer::DoubleBuffer,
};

const BUFFER_SIZE: usize = 256;

#[derive(Component)]
struct FirstBindGroup;

#[derive(Component)]
struct SecondBindGroup;

#[derive(Resource)]
struct IncrementPipeline {
    pipeline_id: CachedComputePipelineId,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct IncrementFirstLabel;

struct IncrementFirstNode;

impl Node for IncrementFirstNode {
    #[allow(clippy::cast_possible_truncation)]
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let increment_pipeline = world.resource::<IncrementPipeline>();

        let Some(mut query) =
            world.try_query_filtered::<&SwappableBindGroup, With<FirstBindGroup>>()
        else {
            return Ok(());
        };

        let Ok(bind_group) = query.single(world) else {
            return Ok(());
        };

        let Some(pipeline) = pipeline_cache.get_compute_pipeline(increment_pipeline.pipeline_id)
        else {
            return Ok(());
        };

        let mut pass =
            render_context
                .command_encoder()
                .begin_compute_pass(&ComputePassDescriptor {
                    label: Some("increment_first_pass"),
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
        render_app.add_systems(
            RenderStartup,
            (setup_render_resources, add_render_graph_node).chain(),
        );
    }
}

fn add_render_graph_node(mut render_graph: ResMut<RenderGraph>) {
    render_graph.add_node(IncrementFirstLabel, IncrementFirstNode);
}

struct ComputeTestResultsPlugin;

impl Plugin for ComputeTestResultsPlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(
            Render,
            check_compute_results
                .in_set(RenderSystems::Cleanup)
                .after(clear_step),
        );
    }
}

fn check_compute_results(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    first_query: Query<&SwappableBindGroup, With<FirstBindGroup>>,
    second_query: Query<&SwappableBindGroup, With<SecondBindGroup>>,
) {
    let Ok(bind_group) = first_query.single() else {
        panic!("Expected exactly one bind group with FirstBindGroup marker");
    };
    let second_count = second_query.iter().count();
    assert_eq!(
        second_count, 1,
        "Expected exactly one bind group with SecondBindGroup marker"
    );

    let Some(read_data) =
        bind_group.read_back_double_buffer_read::<u32>(0, &render_device, &render_queue)
    else {
        panic!("Failed to get data from bind group");
    };
    let Some(write_data) =
        bind_group.read_back_double_buffer_write::<u32>(0, &render_device, &render_queue)
    else {
        panic!("Failed to get data from bind group");
    };

    if read_data[0] == write_data[0] {
        return;
    }

    for (&read_val, &write_val) in read_data.iter().zip(write_data.iter()) {
        assert_eq!(
            read_val,
            write_val + 1,
            "First buffer increment mismatch: read={read_val}, write={write_val}"
        );
    }
}

fn setup_render_resources(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    pipeline_cache: ResMut<PipelineCache>,
    asset_server: Res<AssetServer>,
) {
    let first_data = vec![1u32; BUFFER_SIZE];
    let first_buffer = DoubleBuffer::new(&render_device, &first_data, Some("first_buffer"));

    let second_data = vec![100u32; BUFFER_SIZE];
    let second_buffer = DoubleBuffer::new(&render_device, &second_data, Some("second_buffer"));

    let mut builder = BindGroupBuilder::new();
    builder.add_compute_double(first_buffer);
    let first_swappable = builder.build(&render_device, Some("first_bind_group"));

    let mut builder = BindGroupBuilder::new();
    builder.add_compute_double(second_buffer);
    let second_swappable = builder.build(&render_device, Some("second_bind_group"));

    let shader = asset_server.load("shaders/tests/double_buffer.wgsl");

    let pipeline_id = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("increment_pipeline".into()),
        layout: vec![first_swappable.layout().clone()],
        shader,
        shader_defs: vec![],
        entry_point: Some("main".into()),
        push_constant_ranges: vec![],
        zero_initialize_workgroup_memory: true,
    });

    commands.insert_resource(IncrementPipeline { pipeline_id });

    commands.spawn((first_swappable, FirstBindGroup));
    commands.spawn((second_swappable, SecondBindGroup));
}

#[test]
fn test_multiple_bind_groups_with_markers() {
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
