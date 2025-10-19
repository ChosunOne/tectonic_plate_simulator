use bevy::prelude::*;

use crate::resources::mantle_grid::MantleGrid;

pub fn draw_triangle_grid(mut gizmos: Gizmos, grid: Res<MantleGrid>) {
    let points = grid.sphere.raw_points();
    let indices = grid.sphere.get_all_indices();

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

    for triangle_idx in 0..indices.len() / 3 {
        let center = grid.cells[triangle_idx].center;
        gizmos.cross(center, 0.005, Color::srgb(1.0, 0.0, 0.0));
    }

    for i in 0..grid.cells.len() {
        let center_i = grid.cells[i].center;
        for &neighbor_idx in &grid.neighbors[i] {
            if neighbor_idx > i {
                // Only draw each connection once
                let center_j = grid.cells[neighbor_idx].center;
                gizmos.line(center_i, center_j, Color::srgb(0.0, 0.0, 1.0));
            }
        }
    }
}
