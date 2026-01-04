use std::time::Duration;

use bevy::{
    DefaultPlugins,
    app::{App, Plugin, ScheduleRunnerPlugin},
    ecs::{
        component::Component,
        query::{With, Without},
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
struct FirstPassBindGroup;

#[derive(Component)]
struct SecondPassBindGroup;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct FirstPassLabel;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct SecondPassLabel;

struct ChainedPassesTestPlugin;

impl Plugin for ChainedPassesTestPlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(RenderStartup, setup_chained_passes);
    }
}

#[allow(clippy::cast_possible_truncation)]
fn setup_chained_passes(world: &mut World) {
    let first_data = vec![1u32; BUFFER_SIZE];
    let first_pass = ComputePass::builder()
        .label("first_increment")
        .shader("shaders/tests/double_buffer.wgsl")
        .entry_point("main")
        .workgroups((BUFFER_SIZE as u32).div_ceil(64), 1, 1)
        .double_buffer(first_data)
        .owned_bind_group_marker(FirstPassBindGroup)
        .build(world);

    let second_data = vec![10u32; BUFFER_SIZE];
    let second_pass = ComputePass::builder()
        .label("second_add_from_external")
        .shader("shaders/tests/double_from_external.wgsl")
        .entry_point("main")
        .workgroups((BUFFER_SIZE as u32).div_ceil(64), 1, 1)
        .double_buffer(second_data)
        .owned_bind_group_marker(SecondPassBindGroup)
        .bind_group::<FirstPassBindGroup>(1)
        .build(world);

    let mut render_graph = world.resource_mut::<RenderGraph>();
    render_graph.add_node(FirstPassLabel, first_pass);
    render_graph.add_node(SecondPassLabel, second_pass);
    render_graph.add_node_edge(FirstPassLabel, SecondPassLabel);
}

struct ChainedPassesResultsPlugin;

impl Plugin for ChainedPassesResultsPlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(
            Render,
            check_chained_passes_results
                .in_set(RenderSystems::Cleanup)
                .after(clear_step),
        );
    }
}

fn check_chained_passes_results(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    first_query: Query<&SwappableBindGroup, With<FirstPassBindGroup>>,
    second_query: Query<
        &SwappableBindGroup,
        (With<SecondPassBindGroup>, Without<FirstPassBindGroup>),
    >,
) {
    let Ok(first_bind_group) = first_query.single() else {
        return;
    };
    let Ok(second_bind_group) = second_query.single() else {
        return;
    };

    let Some(first_read) =
        first_bind_group.read_back_double_buffer_read::<u32>(0, &render_device, &render_queue)
    else {
        return;
    };
    let Some(first_write) =
        first_bind_group.read_back_double_buffer_write::<u32>(0, &render_device, &render_queue)
    else {
        return;
    };

    if first_read[0] == first_write[0] {
        return;
    }

    for (&read_val, &write_val) in first_read.iter().zip(first_write.iter()) {
        assert_eq!(
            read_val,
            write_val + 1,
            "First pass should increment: read={read_val}, write={write_val}"
        );
    }

    let Some(second_read) =
        second_bind_group.read_back_double_buffer_read::<u32>(0, &render_device, &render_queue)
    else {
        return;
    };

    let Some(second_write) =
        second_bind_group.read_back_double_buffer_write::<u32>(0, &render_device, &render_queue)
    else {
        return;
    };

    for i in 0..second_read.len() {
        let expected = second_write[i] + first_write[i];
        assert_eq!(
            second_read[i], expected,
            "Second pass should add external: second_read={}, expected second_write({}) + first_write({}) = {}",
            second_read[i], second_write[i], first_write[i], expected
        );
    }
}

#[test]
fn test_compute_pass_chained() {
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
        ChainedPassesTestPlugin,
        ChainedPassesResultsPlugin,
    ));

    app.finish();
    app.cleanup();

    let num_frames = 10;
    for _ in 0..num_frames {
        app.update();
    }
}
