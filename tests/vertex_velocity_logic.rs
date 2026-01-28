use std::f32::consts::{PI, TAU};

use tectonic_plate_simulator::{constants::SPHERE_RADIUS, resources::mesh_grid::MeshGrid};

fn mod_tau(theta: f32) -> f32 {
    (theta + TAU) % TAU
}

fn get_adjacent_edges(
    edge: u32,
    cell: u32,
    edge_cell_indices: &[u32],
    edge_vertex_indices: &[u32],
    cell_edge_indices: &[u32],
) -> [u32; 2] {
    let mut left_edge = 0;
    let mut right_edge = 0;
    let is_secondary = cell == edge_cell_indices[edge as usize * 2 + 1];

    let mut left_vertex = edge_vertex_indices[edge as usize * 2];
    let mut right_vertex = edge_vertex_indices[edge as usize * 2 + 1];

    if is_secondary {
        left_vertex = left_vertex ^ right_vertex;
        right_vertex = left_vertex ^ right_vertex;
        left_vertex = left_vertex ^ right_vertex;
    }

    for i in 0..3 {
        let other_edge = cell_edge_indices[cell as usize * 3 + i];
        if other_edge == edge {
            continue;
        }
        let other_left_vertex = edge_vertex_indices[other_edge as usize * 2];
        let other_right_vertex = edge_vertex_indices[other_edge as usize * 2 + 1];
        if other_left_vertex == left_vertex || other_right_vertex == left_vertex {
            left_edge = other_edge;
        } else if other_left_vertex == right_vertex || other_right_vertex == right_vertex {
            right_edge = other_edge;
        }
    }

    [left_edge, right_edge]
}

fn find_common_cell(edge_idx_a: u32, edge_idx_b: u32, edge_cell_indices: &[u32]) -> u32 {
    let a_primary_cell = edge_cell_indices[edge_idx_a as usize * 2];
    let a_secondary_cell = edge_cell_indices[edge_idx_a as usize * 2 + 1];
    let b_primary_cell = edge_cell_indices[edge_idx_b as usize * 2];
    let b_secondary_cell = edge_cell_indices[edge_idx_b as usize * 2 + 1];

    let cell;
    if a_primary_cell == b_primary_cell || a_primary_cell == b_secondary_cell {
        cell = a_primary_cell;
    } else if a_secondary_cell == b_primary_cell || a_secondary_cell == b_secondary_cell {
        cell = a_secondary_cell;
    } else {
        return 1000000;
    }

    cell
}

fn compute_angles(base: u32, left: u32, right: u32, edge_lengths: &[f32]) -> [f32; 3] {
    let a = edge_lengths[base as usize];
    let b = edge_lengths[left as usize];
    let c = edge_lengths[right as usize];

    let a_squared = a * a;
    let b_squared = b * b;
    let c_squared = c * c;

    let left_base_angle = ((a_squared + b_squared - c_squared) / (2.0 * a * b))
        .clamp(-1.0, 1.0)
        .acos();

    let right_base_angle = ((a_squared + c_squared - b_squared) / (2.0 * a * c))
        .clamp(-1.0, 1.0)
        .acos();

    let apex_angle = ((b_squared + c_squared - a_squared) / (2.0 * b * c))
        .clamp(-1.0, 1.0)
        .acos();

    [left_base_angle, right_base_angle, apex_angle]
}

fn vertex_velocity(
    vertex_idx: usize,
    vertex_edge_offsets: &[u32],
    vertex_edge_indices: &[u32],
    velocity: &[[f32; 2]],
    edge_vertex_indices: &[u32],
    edge_cell_indices: &[u32],
    cell_edge_indices: &[u32],
    edge_lengths: &[f32],
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
        let vel = velocity[edge_idx as usize];
        let mag = vel[0];
        let mut angle = vel[1];

        let v_lower = edge_vertex_indices[edge_idx as usize * 2];

        if vertex_idx as u32 != v_lower {
            angle = mod_tau(angle + PI);
        }

        if i > 0 {
            let cell = find_common_cell(prev_edge_idx, edge_idx, edge_cell_indices);
            let edges = get_adjacent_edges(
                prev_edge_idx,
                cell,
                edge_cell_indices,
                edge_vertex_indices,
                cell_edge_indices,
            );
            let base_edge = edge_idx;
            let right_edge = edges[0];
            let left_edge = prev_edge_idx;
            let angles = compute_angles(base_edge, left_edge, right_edge, edge_lengths);
            angle_increment = mod_tau(angle_increment + angles[0]);
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
    let grid = MeshGrid::new(10);
    let num_edges = grid.edge_cell_adjacency().rows();

    let mut velocity = vec![[0.0, 0.0]; num_edges];

    velocity[1687] = [91.8275, 3.5880];
    velocity[2575] = [76.5344, 2.4911];
    velocity[2751] = [78.8011, 0.9838];
    velocity[1874] = [86.7082, 6.1852];
    velocity[1689] = [73.9837, 4.8234];

    let vertex_idx = 10;
    let mut edge_lengths = vec![0.0f32; grid.edge_cell_adjacency().rows()];
    for (i, length) in edge_lengths.iter_mut().enumerate() {
        let left_vertex_idx = grid.edge_vertex_adjacency().data()[i * 2] as usize;
        let right_vertex_idx = grid.edge_vertex_adjacency().data()[i * 2 + 1] as usize;
        let left_vertex = grid.sphere().raw_points()[left_vertex_idx] * SPHERE_RADIUS;
        let right_vertex = grid.sphere().raw_points()[right_vertex_idx] * SPHERE_RADIUS;
        *length = left_vertex.distance(right_vertex);
    }

    let v_vel = vertex_velocity(
        vertex_idx,
        grid.vertex_edge_adjacency().indptr().as_slice().unwrap(),
        grid.vertex_edge_adjacency().data(),
        &velocity,
        grid.edge_vertex_adjacency().data(),
        grid.edge_cell_adjacency().data(),
        grid.cell_edge_adjacency().data(),
        &edge_lengths,
    );

    dbg!(&v_vel);
}
