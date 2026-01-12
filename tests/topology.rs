use std::time::Duration;

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
    components::render::{SwappableBindGroup, TopologyBindGroup},
    plugins::{mesh_grid::MeshGridPlugin, swappable_bind_group::SwappableBindGroupPlugin},
    resources::mesh_grid::MeshGrid,
};

struct TopologyTestPlugin;

impl Plugin for TopologyTestPlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(Render, verify_topology.in_set(RenderSystems::Cleanup));
    }
}

fn verify_topology(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    grid: Res<MeshGrid>,
    topology_query: Query<&SwappableBindGroup, With<TopologyBindGroup>>,
) {
    let Ok(topology_bg) = topology_query.single() else {
        return;
    };

    let edge_vertex_adjacency = grid.edge_vertex_adjacency();
    let edge_cell_adjacency = grid.edge_cell_adjacency();
    let cell_edge_adjacency = grid.cell_edge_adjacency();

    let num_edges = edge_vertex_adjacency.len();
    let num_cells = grid.cells().len();

    let edge_indices_size = num_edges * 2 * std::mem::size_of::<u32>();
    let Some(edge_vertex_indices) =
        topology_bg.read_back_buffer::<u32>(0, edge_indices_size, &render_device, &render_queue)
    else {
        return;
    };

    assert_eq!(
        &edge_vertex_indices,
        edge_vertex_adjacency.indices(),
        "edge_vertex_indices mismatch"
    );

    let Some(edge_cell_indices) =
        topology_bg.read_back_buffer::<u32>(1, edge_indices_size, &render_device, &render_queue)
    else {
        return;
    };

    assert_eq!(
        &edge_cell_indices,
        edge_cell_adjacency.indices(),
        "edge_cell_indices mismatch"
    );

    let cell_indices_size = num_cells * 3 * std::mem::size_of::<u32>();
    let Some(cell_edge_indices) =
        topology_bg.read_back_buffer::<u32>(2, cell_indices_size, &render_device, &render_queue)
    else {
        return;
    };
    assert_eq!(
        &cell_edge_indices,
        cell_edge_adjacency.indices(),
        "cell_edge_indices mismatch"
    );

    let Some(cell_vertices) =
        topology_bg.read_back_buffer::<u32>(3, cell_indices_size, &render_device, &render_queue)
    else {
        return;
    };

    let expected_cell_vertices = grid
        .cells()
        .iter()
        .flat_map(|cell| cell.vertices)
        .collect::<Vec<u32>>();
    assert_eq!(
        cell_vertices, expected_cell_vertices,
        "cell_vertices mismatch"
    );
}

#[test]
fn test_topology_gpu_buffers() {
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
        MeshGridPlugin,
        TopologyTestPlugin,
    ));

    app.finish();
    app.cleanup();

    let num_frames = 10;
    for _ in 0..num_frames {
        app.update();
    }
}
