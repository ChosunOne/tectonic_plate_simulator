use std::time::Duration;

use bevy::{
    DefaultPlugins,
    app::{App, Plugin, ScheduleRunnerPlugin},
    ecs::{
        query::With,
        schedule::IntoScheduleConfigs,
        system::{Query, Res},
    },
    log::info,
    prelude::PluginGroup,
    render::{
        Render, RenderApp, RenderPlugin, RenderSystems,
        renderer::{RenderDevice, RenderQueue},
    },
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use tectonic_plate_simulator::{
    components::render::{PressureBindGroup, SwappableBindGroup, VertexPressureBindGroup},
    plugins::{
        mantle_grid::MantleGridPlugin,
        pressure::PressurePlugin,
        swappable_bind_group::{SwappableBindGroupPlugin, swap_bind_groups},
        vertex_pressure::VertexPressurePlugin,
    },
    resources::mantle_grid::MantleGrid,
};

struct VertexPressureTestPlugin;

impl Plugin for VertexPressureTestPlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(
            Render,
            verify_vertex_pressure
                .in_set(RenderSystems::Cleanup)
                .before(swap_bind_groups),
        );
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn verify_vertex_pressure(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    grid: Res<MantleGrid>,
    pressure_query: Query<&SwappableBindGroup, With<PressureBindGroup>>,
    vertex_pressure_query: Query<&SwappableBindGroup, With<VertexPressureBindGroup>>,
) {
    let Ok(pressure_bg) = pressure_query.single() else {
        return;
    };

    let Ok(vertex_pressure_bg) = vertex_pressure_query.single() else {
        return;
    };

    let num_vertices = grid.vertex_cell_adjacency().len();
    let Some(vertex_pressure) = vertex_pressure_bg.read_back_buffer::<f32>(
        2,
        num_vertices * std::mem::size_of::<f32>(),
        &render_device,
        &render_queue,
    ) else {
        return;
    };

    let vertex_cell_adjacency = grid.vertex_cell_adjacency();

    let Some(pressure) =
        pressure_bg.read_back_double_buffer_read::<f32>(0, &render_device, &render_queue)
    else {
        return;
    };

    if vertex_pressure[0] == 0.0 {
        return;
    }

    for vertex_idx in 0..num_vertices {
        let expected = vertex_cell_adjacency
            .get(vertex_idx)
            .map(|cell_idx| pressure[cell_idx])
            .sum::<f32>()
            / vertex_cell_adjacency.count(vertex_idx) as f32;

        let actual = vertex_pressure[vertex_idx];
        let diff = (actual - expected).abs();

        assert!(
            diff < 0.001,
            "Vertex {vertex_idx}: expected {expected}, got {actual} (diff: {diff})",
        );
    }
}

#[test]
fn test_vertex_pressure_computation() {
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
        PressurePlugin,
        VertexPressurePlugin,
        VertexPressureTestPlugin,
    ));

    app.finish();
    app.cleanup();

    let num_frames = 15;
    for _ in 0..num_frames {
        app.update();
    }
}
