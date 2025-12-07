use bevy::{
    DefaultPlugins,
    app::{App, Plugin},
    asset::AssetServer,
    ecs::{
        resource::Resource,
        schedule::{IntoScheduleConfigs, common_conditions::run_once},
        system::{Commands, Res, ResMut},
        world::World,
    },
    prelude::PluginGroup,
    render::{
        Render, RenderApp, RenderSystems,
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
    plugins::swappable_bind_group::SwappableBindGroupPlugin,
    render::{
        double_buffer::DoubleBuffer,
        swappable_bind_group::{BindGroupBuilder, SwappableBindGroup},
    },
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
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let increment_pipeline = world.resource::<IncrementPipeline>();
        let bind_group = world.resource::<SwappableBindGroup>();

        let test_buffer = world.resource::<TestDoubleBuffer>();
        let render_queue = world.resource::<RenderQueue>();

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
        // let render_device = render_context.render_device();
        // let read_data = test_buffer
        //     .0
        //     .read_back_read_buffer(render_device, render_queue);
        // let write_data = test_buffer
        //     .0
        //     .read_back_write_buffer(render_device, render_queue);
        //
        // for (&a, &b) in read_data.iter().zip(write_data.iter()) {
        //     assert_eq!(b + 1, a);
        // }

        Ok(())
    }
}

struct ComputeTestPlugin;

impl Plugin for ComputeTestPlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
        render_graph.add_node(IncrementLabel, IncrementNode);
        render_app.add_systems(
            Render,
            setup_render_resources.in_set(RenderSystems::Prepare),
        );
    }
}

fn setup_render_resources(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    mut pipeline_cache: ResMut<PipelineCache>,
    asset_server: Res<AssetServer>,
) {
    let data = vec![0u32; BUFFER_SIZE];
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
    commands.insert_resource(swappable);
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
            .disable::<WinitPlugin>(),
        SwappableBindGroupPlugin,
        ComputeTestPlugin,
    ));

    app.finish();
    app.cleanup();

    let num_frames = 10;
    for _ in 0..num_frames {
        app.update();
    }
}
