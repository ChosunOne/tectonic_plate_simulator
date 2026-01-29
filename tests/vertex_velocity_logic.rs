use std::f32::consts::{PI, TAU};

use tectonic_plate_simulator::{
    LocalFrame, constants::SPHERE_RADIUS, resources::mesh_grid::MeshGrid,
};

fn get_transport_value(
    row_offsets: &[u32],
    col_indices: &[u32],
    values: &[f32],
    row: u32,
    col: u32,
) -> f32 {
    let mut left = row_offsets[row as usize];
    let mut right = row_offsets[(row + 1) as usize] - 1;

    let mut first_true_col = u32::MAX;
    while left <= right {
        let mid = left + (right - left) / 2;
        if col_indices[mid as usize] >= col {
            first_true_col = mid;
            if mid == 0 {
                break;
            }
            right = mid - 1;
        } else {
            left = mid + 1;
        }
    }

    if first_true_col == u32::MAX || col_indices[first_true_col as usize] != col {
        return f32::NAN;
    }

    values[first_true_col as usize]
}

fn mod_tau(theta: f32) -> f32 {
    (theta + TAU) % TAU
}

fn vertex_velocity(
    vertex_idx: usize,
    vertex_edge_offsets: &[u32],
    vertex_edge_indices: &[u32],
    velocity: &[[f32; 2]],
    edge_parallel_transport_row_indices: &[u32],
    edge_parallel_transport_col_indices: &[u32],
    edge_parallel_transport_data: &[f32],
) -> [f32; 2] {
    let start = vertex_edge_offsets[vertex_idx];
    let end = vertex_edge_offsets[vertex_idx + 1];
    let num_edges = end - start;

    if num_edges == 0 {
        return [0.0, 0.0];
    }

    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    let mut angle_increment = 0.0;
    let mut prev_edge_idx = 0;

    for i in 0..num_edges {
        let edge_idx = vertex_edge_indices[start as usize + i as usize];
        dbg!(edge_idx);
        let vel = velocity[edge_idx as usize];
        dbg!(&vel);
        let mag = vel[0];
        let angle = vel[1];

        if i > 0 {
            angle_increment = angle_increment
                + get_transport_value(
                    edge_parallel_transport_row_indices,
                    edge_parallel_transport_col_indices,
                    edge_parallel_transport_data,
                    edge_idx,
                    prev_edge_idx,
                );
        }

        let rotated_angle = mod_tau(angle + angle_increment);

        sum_x += mag * rotated_angle.cos();
        sum_y += mag * rotated_angle.sin();
        prev_edge_idx = edge_idx;
    }

    let avg_x = sum_x / num_edges as f32;
    let avg_y = sum_y / num_edges as f32;

    let avg_mag = (avg_x * avg_x + avg_y * avg_y).sqrt();
    if avg_mag < f32::EPSILON {
        return [0.0, 0.0];
    }

    let avg_angle = mod_tau(avg_y.atan2(avg_x));
    [avg_mag, avg_angle]
}

#[test]
fn test_vertex_velocity_logic() {
    let grid = MeshGrid::new(1);
    let num_edges = grid.edge_cell_adjacency().rows();

    let mut velocity = vec![[0.0, 0.0]; num_edges];

    for i in 0..num_edges {
        let frame = LocalFrame::from_edge(
            i,
            grid.sphere().raw_points(),
            grid.cells(),
            grid.edge_vertex_adjacency(),
            grid.edge_cell_adjacency(),
        );
        let latitude = (frame.origin.y / frame.origin.length()).asin();

        let angle = frame.bearing_to_local_angle(PI / 2.0);
        let magnitude = 100.0 * latitude.cos();

        velocity[i] = [magnitude, angle];
    }

    let vertex_idx = *grid.edge_vertex_adjacency().get(44, 0).unwrap() as usize;
    dbg!(vertex_idx);

    let v_vel = vertex_velocity(
        vertex_idx,
        grid.vertex_edge_adjacency().indptr().as_slice().unwrap(),
        grid.vertex_edge_adjacency().data(),
        &velocity,
        grid.edge_parallel_transport().indptr().as_slice().unwrap(),
        grid.edge_parallel_transport().indices(),
        grid.edge_parallel_transport().data(),
    );

    dbg!(&grid.vertex_angle_offsets()[vertex_idx]);

    dbg!(&v_vel);
}
