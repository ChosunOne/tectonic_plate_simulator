use std::{
    f32::consts::{PI, TAU},
    time::Duration,
};

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
    components::render::{
        SwappableBindGroup, VelocityBindGroup, VertexVelocityBindGroup,
        VertexVelocityReductionBindGroup,
    },
    plugins::{
        divergence::DivergencePlugin,
        mantle_grid::MantleGridPlugin,
        swappable_bind_group::{SwappableBindGroupPlugin, swap_bind_groups},
        velocity::VelocityPlugin,
        vertex_velocity::VertexVelocityPlugin,
    },
    resources::mantle_grid::MantleGrid,
};

struct VertexVelocityTestPlugin;

impl Plugin for VertexVelocityTestPlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(
            Render,
            verify_vertex_velocity
                .in_set(RenderSystems::Cleanup)
                .before(swap_bind_groups),
        );
    }
}

fn verify_vertex_velocity(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    grid: Res<MantleGrid>,
    velocity_query: Query<&SwappableBindGroup, With<VelocityBindGroup>>,
    vertex_velocity_query: Query<&SwappableBindGroup, With<VertexVelocityBindGroup>>,
    vertex_velocity_reduction_query: Query<
        &SwappableBindGroup,
        With<VertexVelocityReductionBindGroup>,
    >,
) {
    let Ok(velocity_bg) = velocity_query.single() else {
        return;
    };

    let Ok(vertex_velocity_bg) = vertex_velocity_query.single() else {
        return;
    };

    let Ok(vertex_velocity_reduction_bg) = vertex_velocity_reduction_query.single() else {
        return;
    };

    let vertex_edge_adjacency = grid.vertex_edge_adjacency();
    let edge_vertex_adjacency = grid.edge_vertex_adjacency();
    let num_vertices = vertex_edge_adjacency.len();

    let Some(edge_velocities) =
        velocity_bg.read_back_double_buffer_read::<[f32; 2]>(0, &render_device, &render_queue)
    else {
        return;
    };

    let Some(vertex_velocity) = vertex_velocity_bg.read_back_buffer::<[f32; 2]>(
        2,
        num_vertices * std::mem::size_of::<[f32; 2]>(),
        &render_device,
        &render_queue,
    ) else {
        return;
    };

    let all_zero = vertex_velocity.iter().all(|v| v[0] == 0.0);
    if all_zero {
        return;
    }

    for vertex_idx in 0..num_vertices {
        let edges = vertex_edge_adjacency.get(vertex_idx).collect::<Vec<_>>();
        let num_edges = edges.len();

        let angle_increment = TAU / num_edges as f32;

        let mut sum_x = 0.0;
        let mut sum_y = 0.0;

        for (i, &edge_idx) in edges.iter().enumerate() {
            let [mag, mut angle] = edge_velocities[edge_idx];
            let edge_verts = edge_vertex_adjacency.get(edge_idx).collect::<Vec<_>>();
            let v_lower = edge_verts[0];

            if vertex_idx != v_lower {
                angle += PI;
            }

            let rotated_angle = angle + (i as f32) * angle_increment;

            sum_x += mag * rotated_angle.cos();
            sum_y += mag * rotated_angle.sin();
        }

        let expected_mag = (sum_x * sum_x + sum_y * sum_y).sqrt() / num_edges as f32;
        let expected_angle = {
            let avg_x = sum_x / num_edges as f32;
            let avg_y = sum_y / num_edges as f32;
            avg_y.atan2(avg_x)
        };

        let [actual_mag, actual_angle] = vertex_velocity[vertex_idx];

        let mag_diff = (actual_mag - expected_mag).abs();
        assert!(
            mag_diff < 0.01,
            "Vertex {vertex_idx}: magnitude expected {expected_mag}, got {actual_mag} (diff: {mag_diff})"
        );

        let angle_diff = (actual_angle - expected_angle).abs();
        let angle_diff = angle_diff.min(TAU - angle_diff);

        if expected_mag > 0.1 {
            assert!(
                angle_diff < 0.01,
                "Vertex {vertex_idx}: angle expected {expected_angle}, got {actual_angle} (diff: {angle_diff})"
            );
        }
    }

    let Some(velocity_bounds) =
        vertex_velocity_reduction_bg.read_back_buffer::<f32>(1, 2, &render_device, &render_queue)
    else {
        return;
    };

    let magnitudes = vertex_velocity.iter().map(|v| v[0]).collect::<Vec<f32>>();

    let expected_min = magnitudes.iter().copied().fold(f32::MAX, f32::min);
    let expected_max = magnitudes.iter().copied().fold(f32::MIN, f32::max);

    let actual_min = velocity_bounds[0];
    let actual_max = velocity_bounds[1];

    let min_diff = (actual_min - expected_min).abs();
    let max_diff = (actual_max - expected_max).abs();

    assert!(
        min_diff < 0.001,
        "Min magnitude: expected {expected_min}, got {actual_min} (diff: {min_diff})"
    );
    assert!(
        max_diff < 0.001,
        "Max magnitude: expected {expected_max}, got {actual_max} (diff: {max_diff})"
    );
}

#[test]
fn test_vertex_velocity_computation() {
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
        DivergencePlugin,
        VertexVelocityPlugin,
        VertexVelocityTestPlugin,
    ));

    app.finish();
    app.cleanup();

    let num_frames = 30;
    for _ in 0..num_frames {
        app.update();
    }
}
