use std::time::Duration;

use bevy::{
    DefaultPlugins,
    app::{App, Plugin, ScheduleRunnerPlugin},
    ecs::{
        component::Component,
        query::With,
        schedule::IntoScheduleConfigs,
        system::{Query, Res},
        world::World,
    },
    prelude::PluginGroup,
    render::{
        Render, RenderApp, RenderPlugin, RenderStartup, RenderSystems,
        render_graph::{RenderGraph, RenderLabel},
        renderer::{RenderDevice, RenderQueue},
    },
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use tectonic_plate_simulator::{
    components::render::{SwappableBindGroup, compute_pass::ComputePass},
    plugins::swappable_bind_group::{SwappableBindGroupPlugin, clear_step},
};

const BUFFER_SIZE: usize = 256;

#[derive(Component)]
struct TestBindGroup;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct TestPassLabel;

struct TestPlugin;

impl Plugin for TestPlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(RenderStartup, setup_test);
    }
}

#[allow(clippy::cast_possible_truncation)]
fn setup_test(world: &mut World) {
    let data = vec![1u32; BUFFER_SIZE];
    let pass = ComputePass::builder()
        .label("test_increment")
        .shader("shaders/tests/double_buffer.wgsl")
        .entry_point("main")
        .workgroups((BUFFER_SIZE as u32).div_ceil(64), 1, 1)
        .double_buffer(data)
        .owned_bind_group_marker(TestBindGroup)
        .build(world);

    let mut render_graph = world.resource_mut::<RenderGraph>();
    render_graph.add_node(TestPassLabel, pass);
}

struct TestResultsPlugin;

impl Plugin for TestResultsPlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(
            Render,
            check_results
                .in_set(RenderSystems::Cleanup)
                .after(clear_step),
        );
    }
}

fn check_results(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    query: Query<&SwappableBindGroup, With<TestBindGroup>>,
) {
    let Ok(bind_group) = query.single() else {
        return;
    };

    let Some(read_data) =
        bind_group.read_back_double_buffer_read::<u32>(0, &render_device, &render_queue)
    else {
        return;
    };

    let Some(write_data) =
        bind_group.read_back_double_buffer_write::<u32>(0, &render_device, &render_queue)
    else {
        return;
    };

    if read_data[0] == write_data[0] {
        return;
    }

    for (&read_val, &write_val) in read_data.iter().zip(write_data.iter()) {
        assert_eq!(
            read_val,
            write_val + 1,
            "Shader should increment: read={read_val}, write={write_val}"
        );
    }
}

#[test]
fn test_compute_pass_single() {
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
        TestPlugin,
        TestResultsPlugin,
    ));

    app.finish();
    app.cleanup();

    let num_frames = 10;
    for _ in 0..num_frames {
        app.update();
    }
}
