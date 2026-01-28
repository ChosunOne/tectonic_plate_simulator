use bevy::prelude::*;

use crate::{
    LocalFrame,
    constants::SPHERE_RADIUS,
    resources::{
        departure_info::DepartureInfoSync, gizmo_visibility::GizmoVisibility, mesh_grid::MeshGrid,
        selected_edge::SelectedEdge, velocity::VelocitySync, vertex_velocity::VertexVelocitySync,
    },
};

pub fn draw_triangle_grid(
    mut gizmos: Gizmos,
    grid: Res<MeshGrid>,
    visibility: Res<GizmoVisibility>,
) {
    if !visibility.triangle_grid {
        return;
    }
    let points = grid.sphere().raw_points();
    let indices = grid.sphere().get_all_indices();

    for triangle in indices.chunks(3) {
        let (a, b, c) = (
            triangle[0] as usize,
            triangle[1] as usize,
            triangle[2] as usize,
        );

        let pa = SPHERE_RADIUS * points[a];
        let pb = SPHERE_RADIUS * points[b];
        let pc = SPHERE_RADIUS * points[c];

        gizmos.line(pa.into(), pb.into(), Color::srgb(0.0, 1.0, 0.5));
        gizmos.line(pb.into(), pc.into(), Color::srgb(0.0, 1.0, 0.5));
        gizmos.line(pc.into(), pa.into(), Color::srgb(0.0, 1.0, 0.5));
    }
}

pub fn draw_triangle_grid_centers(
    mut gizmos: Gizmos,
    grid: Res<MeshGrid>,
    visibility: Res<GizmoVisibility>,
) {
    if !visibility.triangle_centers {
        return;
    }
    let indices = grid.sphere().get_all_indices();
    for triangle_idx in 0..indices.len() / 3 {
        let center = grid.cells()[triangle_idx].center;
        gizmos.cross(center, 5.00, Color::srgb(1.0, 0.0, 0.0));
    }
}

pub fn draw_triangle_grid_neighbors(
    mut gizmos: Gizmos,
    grid: Res<MeshGrid>,
    visibility: Res<GizmoVisibility>,
) {
    if !visibility.triangle_neighbors {
        return;
    }
    for i in 0..grid.cells().len() {
        let center_i = grid.cells()[i].center;
        for neighbor_idx in grid
            .cell_adjacency()
            .outer_view(i)
            .expect("to have cells for cell")
            .iter()
            .map(|(_, &x)| x as usize)
        {
            if neighbor_idx > i {
                // Only draw each connection once
                let center_j = grid.cells()[neighbor_idx].center;
                gizmos.line(center_i, center_j, Color::srgb(0.0, 0.0, 1.0));
            }
        }
    }
}

pub fn draw_velocity_arrows(
    mut gizmos: Gizmos,
    grid: Res<MeshGrid>,
    velocity_sync: Res<VelocitySync>,
    visibility: Res<GizmoVisibility>,
    selected_edge: Res<SelectedEdge>,
) {
    if !visibility.velocity_arrows {
        return;
    }

    let Ok(velocity) = velocity_sync.0.lock() else {
        return;
    };

    if velocity.is_empty() {
        return;
    }

    let num_edges = grid.edge_cell_adjacency().rows();

    for edge_idx in 0..num_edges {
        let frame = LocalFrame::from_edge(&grid, edge_idx);
        let [magnitude, angle] = velocity[edge_idx];

        let is_selected = selected_edge.0 == Some(edge_idx);

        let color = if is_selected {
            Color::srgb(0.0, 1.0, 1.0)
        } else if magnitude < 10.0 {
            Color::srgb(1.0, 0.5, 0.0)
        } else {
            Color::srgb(1.0, 1.0, 0.0)
        };
        let magnitude = magnitude.max(10.0);

        let arrow_end = frame.polar_to_world_position(magnitude, angle);

        gizmos.arrow(frame.origin, arrow_end, color);
    }
}

pub fn draw_vertex_velocity_arrows(
    mut gizmos: Gizmos,
    grid: Res<MeshGrid>,
    vertex_velocity_sync: Res<VertexVelocitySync>,
    visibility: Res<GizmoVisibility>,
) {
    if !visibility.vertex_velocity_arrows {
        return;
    }

    let Ok(vertex_velocity) = vertex_velocity_sync.0.lock() else {
        return;
    };

    if vertex_velocity.is_empty() {
        return;
    }

    let num_vertices = grid.vertex_edge_adjacency().rows();

    for vertex_idx in 0..num_vertices {
        let [mut magnitude, angle] = vertex_velocity[vertex_idx];
        let frame = LocalFrame::from_vertex(&grid, vertex_idx);

        let color = if magnitude > 0.01 {
            Color::srgb(0.0, 0.0, 1.0)
        } else {
            Color::srgb(1.0, 0.0, 0.0)
        };

        magnitude = magnitude.max(10.0);

        let arrow_end = frame.polar_to_world_position(magnitude, angle);

        gizmos.arrow(frame.origin, arrow_end, color);
    }
}

pub fn draw_departure_gizmo(
    mut gizmos: Gizmos,
    grid: Res<MeshGrid>,
    departure_sync: Res<DepartureInfoSync>,
    selected_edge: Res<SelectedEdge>,
) {
    let Some(edge_idx) = selected_edge.0 else {
        return;
    };

    let Ok(departure_data) = departure_sync.0.lock() else {
        return;
    };

    if departure_data.is_empty() || edge_idx >= departure_data.len() {
        return;
    }

    let info = &departure_data[edge_idx];

    if info.pos[0] < f32::EPSILON && info.interpolated_velocity[0] < f32::EPSILON {
        return;
    }

    let base_edge_idx = info.base_edge as usize;

    let frame = LocalFrame::from_edge(&grid, base_edge_idx);
    let departure_pos = frame.polar_to_world_position(info.pos[0], info.pos[1]);
    let offset = departure_pos - frame.origin;
    let interpolated_velocity =
        frame.polar_to_world_position(info.interpolated_velocity[0], info.interpolated_velocity[1]);
    let last_velocity = frame.polar_to_world_position(info.last_velocity[0], info.last_velocity[1]);

    gizmos.cross(
        departure_pos.normalize() * SPHERE_RADIUS,
        2.0,
        Color::srgb(0.0, 1.0, 1.0),
    );
    gizmos.arrow(
        departure_pos.normalize() * SPHERE_RADIUS,
        interpolated_velocity + offset,
        Color::srgb(1.0, 0.0, 1.0),
    );
    gizmos.arrow(frame.origin, last_velocity, Color::srgb(1.0, 0.0, 0.0));
}
