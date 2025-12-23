use bevy::prelude::*;

use crate::resources::{
    gizmo_visibility::GizmoVisibility, mantle_grid::MantleGrid, velocity::VelocitySync,
    vertex_velocity::VertexVelocitySync,
};

pub fn draw_triangle_grid(
    mut gizmos: Gizmos,
    grid: Res<MantleGrid>,
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

        let pa = points[a];
        let pb = points[b];
        let pc = points[c];

        gizmos.line(pa.into(), pb.into(), Color::srgb(0.0, 1.0, 0.5));
        gizmos.line(pb.into(), pc.into(), Color::srgb(0.0, 1.0, 0.5));
        gizmos.line(pc.into(), pa.into(), Color::srgb(0.0, 1.0, 0.5));
    }
}

pub fn draw_triangle_grid_centers(
    mut gizmos: Gizmos,
    grid: Res<MantleGrid>,
    visibility: Res<GizmoVisibility>,
) {
    if !visibility.triangle_centers {
        return;
    }
    let indices = grid.sphere().get_all_indices();
    for triangle_idx in 0..indices.len() / 3 {
        let center = grid.cells()[triangle_idx].center;
        gizmos.cross(center, 0.005, Color::srgb(1.0, 0.0, 0.0));
    }
}

pub fn draw_triangle_grid_neighbors(
    mut gizmos: Gizmos,
    grid: Res<MantleGrid>,
    visibility: Res<GizmoVisibility>,
) {
    if !visibility.triangle_neighbors {
        return;
    }
    for i in 0..grid.cells().len() {
        let center_i = grid.cells()[i].center;
        for neighbor_idx in grid.cell_adjacency().get(i) {
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
    grid: Res<MantleGrid>,
    velocity_sync: Res<VelocitySync>,
    visibility: Res<GizmoVisibility>,
) {
    if !visibility.velocity_arrows {
        return;
    }

    let Ok(velocity) = velocity_sync.0.lock() else {
        return;
    };

    let points = grid.sphere().raw_points();
    let edge_vertex_adjacency = grid.edge_vertex_adjacency();
    let edge_cell_adjacency = grid.edge_cell_adjacency();

    let scale = 0.00005;

    for edge_idx in 0..edge_vertex_adjacency.len() {
        let edge_verts = edge_vertex_adjacency.get(edge_idx).collect::<Vec<_>>();
        let v_lower_pos: Vec3 = points[edge_verts[0]].into();
        let v_higher_pos: Vec3 = points[edge_verts[1]].into();

        let midpoint = (v_lower_pos + v_higher_pos) / 2.0;
        let edge_dir = (v_higher_pos - v_lower_pos).normalize();

        let toward_v_lower = -edge_dir;
        let surface_normal = midpoint.normalize();
        let perp = surface_normal.cross(edge_dir).normalize();

        let edge_cells = edge_cell_adjacency.get(edge_idx).collect::<Vec<_>>();
        let primary_cell_center = grid.cells()[edge_cells[0]].center;
        let to_primary = primary_cell_center - midpoint;
        let toward_primary = if to_primary.dot(perp) > 0.0 {
            perp
        } else {
            -perp
        };

        let [magnitude, angle] = velocity[edge_idx];
        let direction = angle.cos() * toward_v_lower + angle.sin() * toward_primary;

        let arrow_end = midpoint + direction * magnitude * scale;

        gizmos.arrow(midpoint, arrow_end, Color::srgb(1.0, 1.0, 0.0));
    }
}

pub fn draw_vertex_velocity_arrows(
    mut gizmos: Gizmos,
    grid: Res<MantleGrid>,
    vertex_velocity_sync: Res<VertexVelocitySync>,
    visibility: Res<GizmoVisibility>,
) {
    if !visibility.vertex_velocity_arrows {
        return;
    }

    let Ok(vertex_velocity) = vertex_velocity_sync.0.lock() else {
        return;
    };

    let points = grid.sphere().raw_points();
    let vertex_edge_adjacency = grid.vertex_edge_adjacency();
    let edge_vertex_adjacency = grid.edge_vertex_adjacency();

    let scale = 0.00005;

    for vertex_idx in 0..vertex_edge_adjacency.len() {
        let [magnitude, angle] = vertex_velocity[vertex_idx];
        let vertex_pos: Vec3 = points[vertex_idx].into();
        let vertex_normal = vertex_pos.normalize();

        let edge_0_idx = vertex_edge_adjacency
            .get(vertex_idx)
            .next()
            .expect("to have edge of vertex");
        let edge_0_verts = edge_vertex_adjacency.get(edge_0_idx).collect::<Vec<_>>();
        let v_other = if edge_0_verts[0] == vertex_idx {
            edge_0_verts[1]
        } else {
            edge_0_verts[0]
        };

        let other_pos: Vec3 = points[v_other].into();
        let toward_self = (vertex_pos - other_pos).normalize();

        let tangent_x = (toward_self - vertex_normal * toward_self.dot(vertex_normal)).normalize();
        let tangent_y = tangent_x.cross(vertex_normal).normalize();

        let direction = angle.cos() * tangent_x + angle.sin() * tangent_y;

        let arrow_end = vertex_pos + direction * magnitude * scale;

        gizmos.arrow(vertex_pos, arrow_end, Color::srgb(0.0, 0.0, 1.0));
    }
}
