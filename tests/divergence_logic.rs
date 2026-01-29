use std::f32::consts::{PI, TAU};

use tectonic_plate_simulator::{LocalFrame, resources::mesh_grid::MeshGrid};

const RHO: f32 = 1.0;
const DT: f32 = 1.0 / 60.0;

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
    if theta >= 0.0 && theta < TAU {
        return theta;
    }
    (theta + TAU) % TAU
}

fn is_primary(cell_idx: u32, edge_idx: u32, edge_cell_data: &[u32]) -> bool {
    return cell_idx == edge_cell_data[edge_idx as usize * 2];
}

fn divergence(
    cell_idx: u32,
    cell_edge_data: &[u32],
    edge_cell_data: &[u32],
    velocity: &[[f32; 2]],
    edge_parallel_transport_row_indices: &[u32],
    edge_parallel_transport_col_indices: &[u32],
    edge_parallel_transport_data: &[f32],
    edge_lengths: &[f32],
) -> f32 {
    let mut sum = 0.0;
    let mut base_edge_idx = u32::MAX;
    for i in 0..3 {
        let edge_idx = cell_edge_data[cell_idx as usize * 3 + i];
        dbg!(edge_idx);
        if is_primary(cell_idx, edge_idx, edge_cell_data) {
            base_edge_idx = edge_idx;
            break;
        }
    }

    for i in 0..3 {
        let edge_idx = cell_edge_data[cell_idx as usize * 3 + i];
        let edge_velocity = velocity[edge_idx as usize];
        dbg!(edge_idx);
        dbg!(&edge_velocity);
        let mag = edge_velocity[0];
        let mut angle = edge_velocity[1];
        if !is_primary(cell_idx, edge_idx, edge_cell_data) {
            angle = mod_tau(angle + PI);
        }
        dbg!(angle);
        if angle.sin().abs() < f32::EPSILON || mag.abs() < f32::EPSILON {
            continue;
        }
        sum = sum + mag * angle.sin() * edge_lengths[edge_idx as usize];
    }

    sum = RHO * sum / (DT * cell_area(cell_idx, edge_lengths, cell_edge_data));

    if sum.abs() < f32::EPSILON {
        return 0.0;
    }

    return sum;
}

fn cell_area(cell: u32, edge_lengths: &[f32], cell_edge_data: &[u32]) -> f32 {
    let base_edge = cell_edge_data[cell as usize * 3];
    let left_edge = cell_edge_data[cell as usize * 3 + 1];
    let right_edge = cell_edge_data[cell as usize * 3 + 2];

    let a = edge_lengths[base_edge as usize];
    let b = edge_lengths[left_edge as usize];
    let c = edge_lengths[right_edge as usize];

    let s = (a + b + c) / 2.0;

    return (s * (s - a) * (s - b) * (s - c)).sqrt();
}

#[test]
fn test_divergence_logic() {
    let grid = MeshGrid::new(5);
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

    let edge_idx = 181;
    let cell_idx = *grid.edge_cell_adjacency().get(edge_idx, 0).unwrap();
    dbg!(cell_idx);
    let divergence_value = divergence(
        cell_idx,
        grid.cell_edge_adjacency().data(),
        grid.edge_cell_adjacency().data(),
        &velocity,
        grid.edge_parallel_transport().indptr().as_slice().unwrap(),
        grid.edge_parallel_transport().indices(),
        grid.edge_parallel_transport().data(),
        grid.edge_lengths(),
    );
    dbg!(divergence_value);
    todo!()
}
