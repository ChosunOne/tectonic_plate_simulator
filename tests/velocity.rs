use std::{f32::consts::TAU, time::Duration};

use bevy::{
    DefaultPlugins,
    app::{App, Plugin, ScheduleRunnerPlugin},
    ecs::{
        query::With,
        schedule::IntoScheduleConfigs,
        system::{Query, Res},
    },
    prelude::PluginGroup,
    render::{
        Render, RenderApp, RenderPlugin, RenderSystems,
        renderer::{RenderDevice, RenderQueue},
    },
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use tectonic_plate_simulator::{
    components::render::{SwappableBindGroup, VelocityBindGroup},
    plugins::{
        mantle_grid::MantleGridPlugin,
        swappable_bind_group::{SwappableBindGroupPlugin, swap_bind_groups},
        velocity::VelocityPlugin,
    },
    resources::mantle_grid::MantleGrid,
};

struct VelocityTestPlugin;

impl Plugin for VelocityTestPlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(
            Render,
            verify_velocity
                .in_set(RenderSystems::Cleanup)
                .before(swap_bind_groups),
        );
    }
}

#[allow(clippy::cast_precision_loss)]
fn verify_velocity(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    grid: Res<MantleGrid>,
    velocity_query: Query<&SwappableBindGroup, With<VelocityBindGroup>>,
) {
    let Ok(velocity_bg) = velocity_query.single() else {
        return;
    };

    let num_edges = grid.edge_cell_adjacency().len();

    let Some(velocity) =
        velocity_bg.read_back_double_buffer_read::<f32>(0, &render_device, &render_queue)
    else {
        return;
    };

    for edge_idx in 0..num_edges {
        let expected_magnitude = 500.0;
        let expected_angle = 0.5;

        let actual_magnitude = velocity[edge_idx * 2];
        let actual_angle = velocity[edge_idx * 2 + 1];

        let magnitude_diff = (actual_magnitude - expected_magnitude).abs();
        let angle_diff = (actual_angle - expected_angle).abs();

        assert!(
            magnitude_diff < 0.001,
            "Edge {edge_idx} magnitude mismatch: expected {expected_magnitude}, got {actual_magnitude}"
        );
        assert!(
            angle_diff < 0.001,
            "Edge {edge_idx} angle mismatch: expected {expected_angle}, got {actual_angle}"
        );
    }
}

#[test]
fn test_edge_velocity_gpu_buffers() {
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
        MantleGridPlugin,
        VelocityPlugin,
        VelocityTestPlugin,
    ));

    app.finish();
    app.cleanup();

    let num_frames = 10;
    for _ in 0..num_frames {
        app.update();
    }
}
