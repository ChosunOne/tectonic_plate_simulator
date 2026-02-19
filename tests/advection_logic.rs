use std::f32::consts::{PI, TAU};

use tectonic_plate_simulator::{constants::SPHERE_RADIUS, resources::mesh_grid::MeshGrid};

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

fn add_velocity(vel_a: [f32; 2], vel_b: [f32; 2]) -> [f32; 2] {
    if vel_a[0] < f32::EPSILON {
        return vel_b;
    }
    if vel_b[0] < f32::EPSILON {
        return vel_a;
    }

    let ax = vel_a[0] * vel_a[1].cos();
    let ay = vel_a[0] * vel_a[1].sin();
    let bx = vel_b[0] * vel_b[1].cos();
    let by = vel_b[0] * vel_b[1].sin();

    let rx = ax + bx;
    let ry = ay + by;

    let mag = (rx * rx + ry * ry).sqrt();
    if mag < f32::EPSILON {
        return [0.0, 0.0];
    }

    let angle = ry.atan2(rx);
    [mag, mod_tau(angle)]
}

fn interpolate_velocity(q: f32, vel_a: [f32; 2], vel_b: [f32; 2]) -> [f32; 2] {
    if q.abs() < f32::EPSILON {
        return vel_a;
    }
    if (1.0 - q).abs() < f32::EPSILON {
        return vel_b;
    }
    let va = [(1.0 - q) * vel_a[0], vel_a[1]];
    let vb = [q * vel_b[0], vel_b[1]];
    add_velocity(va, vb)
}

fn interpolate_velocity_with_offsets(
    q: f32,
    vel_a: [f32; 2],
    vel_b: [f32; 2],
    vel_a_offset: f32,
    vel_b_offset: f32,
) -> [f32; 2] {
    interpolate_velocity(
        q,
        [vel_a[0], mod_tau(vel_a_offset + vel_a[1])],
        [vel_b[0], mod_tau(vel_b_offset + vel_b[1])],
    )
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

fn compute_angle_to_apex_vertex(
    d: f32,
    base_edge: u32,
    angles: [f32; 3],
    edge_lengths: &[f32],
) -> f32 {
    let base_edge_length = edge_lengths[base_edge as usize];
    (angles[0].sin() * angles[1].sin()).atan2(
        (d / base_edge_length) * (angles[0] + angles[1]).sin() - angles[1].sin() * angles[0].cos(),
    )
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

fn subcell_crossing_distance(
    theta: f32,
    d: f32,
    base_edge: u32,
    cell: u32,
    edge_cell_indices: &[u32],
    edge_vertex_indices: &[u32],
    cell_edge_indices: &[u32],
    edge_lengths: &[f32],
) -> [f32; 2] {
    let adjacent_edges = get_adjacent_edges(
        base_edge,
        cell,
        edge_cell_indices,
        edge_vertex_indices,
        cell_edge_indices,
    );

    let left_edge = adjacent_edges[0];
    let right_edge = adjacent_edges[1];

    let base_edge_length = edge_lengths[base_edge as usize];

    let angles = compute_angles(base_edge, left_edge, right_edge, edge_lengths);
    let critical_angle = compute_angle_to_apex_vertex(d, base_edge, angles, edge_lengths);
    let left_base_angle = angles[0];
    let right_base_angle = angles[1];

    let mut l_exit = 0.0;
    let mut d_exit = 0.0;

    if theta > f32::EPSILON && theta <= critical_angle {
        dbg!("A");
        let denom = (theta + left_base_angle).sin();
        if denom.abs() < f32::EPSILON || left_base_angle.sin().abs() < f32::EPSILON {
            return [0.0, 0.0];
        }
        l_exit = d * left_base_angle.sin() / denom;
        d_exit = l_exit * theta.sin() / left_base_angle.sin();
    } else if theta > f32::EPSILON && theta < PI {
        dbg!("B");
        let denom = (theta - right_base_angle).sin();
        if denom.abs() < f32::EPSILON || right_base_angle.sin().abs() < f32::EPSILON {
            return [0.0, 0.0];
        }
        l_exit = (base_edge_length - d) * right_base_angle.sin() / denom;
        d_exit = l_exit * theta.sin() / right_base_angle.sin();
    } else if theta.abs() < f32::EPSILON {
        dbg!("C");
        l_exit = d;
        d_exit = 0.0;
    } else if (theta - PI) <= f32::EPSILON {
        dbg!("D");
        l_exit = base_edge_length - d;
        d_exit = 0.0;
    } else {
        dbg!("E");
    }

    [l_exit, d_exit]
}

fn map_to_reference_frame(d: f32, edge: u32, velocity: [f32; 2], edge_lengths: &[f32]) -> [f32; 2] {
    let base_edge_length = edge_lengths[edge as usize];
    let midpoint = base_edge_length / 2.0;

    let midpoint_offset = midpoint - d;
    let x = midpoint_offset + velocity[0] * velocity[1].cos();
    let y = velocity[0] * velocity[1].sin();

    let mag = (x * x + y * y).sqrt();
    if mag < f32::EPSILON {
        return [0.0, 0.0];
    }

    let theta = mod_tau(y.atan2(x));
    [mag, theta]
}

fn interpolate_edge_velocities(
    pos: [f32; 2],
    angles: [f32; 3],
    edges: [u32; 3],
    edge_lengths: &[f32],
    velocity_in: &[[f32; 2]],
    edge_transport_row_indices: &[u32],
    edge_transport_col_indices: &[u32],
    edge_transport_data: &[f32],
) -> [f32; 2] {
    let mut pos_y = pos[1];
    let pos_x = pos[0];
    dbg!(pos_x);
    dbg!(pos_y);
    dbg!(pos_y - PI);
    let mirror_result = pos_y >= PI;
    if mirror_result {
        dbg!("MIRROR");
        pos_y = TAU - pos_y;
    }
    let base_edge_length = edge_lengths[edges[0] as usize];
    let left_edge_length = edge_lengths[edges[1] as usize];
    let right_edge_length = edge_lengths[edges[2] as usize];

    let base_midpoint = base_edge_length / 2.0;
    let left_midpoint = left_edge_length / 2.0;
    let right_midpoint = right_edge_length / 2.0;

    let base_velocity = velocity_in[edges[0] as usize];
    let left_velocity = velocity_in[edges[1] as usize];
    let right_velocity = velocity_in[edges[2] as usize];

    let base_offset = 0.0;

    let to_left_offset = get_transport_value(
        edge_transport_row_indices,
        edge_transport_col_indices,
        edge_transport_data,
        edges[0],
        edges[1],
    );
    let from_left_offset = get_transport_value(
        edge_transport_row_indices,
        edge_transport_col_indices,
        edge_transport_data,
        edges[1],
        edges[0],
    );

    let to_right_offset = get_transport_value(
        edge_transport_row_indices,
        edge_transport_col_indices,
        edge_transport_data,
        edges[0],
        edges[2],
    );
    let from_right_offset = get_transport_value(
        edge_transport_row_indices,
        edge_transport_col_indices,
        edge_transport_data,
        edges[2],
        edges[0],
    );

    let p_ab = base_midpoint - pos_x * pos_y.cos();

    let d_1 = pos_x * pos_y.sin();
    dbg!(d_1);
    if d_1.abs() < f32::EPSILON {
        dbg!("DEGEN 1");
        if p_ab < base_midpoint {
            dbg!("A");
            let q = (p_ab + left_midpoint) / (base_midpoint + left_midpoint);
            return interpolate_velocity_with_offsets(
                q,
                left_velocity,
                base_velocity,
                from_left_offset,
                base_offset,
            );
        }
        dbg!("B");
        let q = (p_ab - base_midpoint) / (base_midpoint + right_midpoint);
        dbg!(q);
        dbg!(base_offset);
        dbg!(from_right_offset);
        return interpolate_velocity_with_offsets(
            q,
            base_velocity,
            right_velocity,
            base_offset,
            from_right_offset,
        );
    }

    let d_a = (p_ab * p_ab + d_1 * d_1).sqrt();
    let phi_a = ((d_1 * d_1 + d_a * d_a - p_ab * p_ab) / (2.0 * d_1 * d_a))
        .clamp(-1.0, 1.0)
        .acos();
    let phi_b = TAU / 4.0 - phi_a;
    let phi_c = angles[0] - phi_b;

    let p_ca = left_edge_length - d_a * phi_c.cos();
    let d_3 = d_a * phi_c.sin();
    if d_3.abs() < f32::EPSILON {
        dbg!("DEGEN 3");
        if p_ca < left_midpoint {
            let q = (p_ca + right_midpoint) / (left_midpoint + right_midpoint);
            return interpolate_velocity_with_offsets(
                q,
                right_velocity,
                left_velocity,
                from_right_offset,
                from_left_offset,
            );
        }
        let q = (p_ca - left_midpoint) / (left_midpoint + base_midpoint);
        return interpolate_velocity_with_offsets(
            q,
            left_velocity,
            base_velocity,
            from_left_offset,
            base_offset,
        );
    }

    let d_c = (p_ca * p_ca + d_3 * d_3).sqrt();
    let d_b = ((base_edge_length - p_ab) * (base_edge_length - p_ab) + d_1 * d_1).sqrt();
    let s = (d_b + d_c + right_edge_length) / 2.0;
    let a = (s * (s - d_b) * (s - d_c) * (s - right_edge_length))
        .max(0.0)
        .sqrt();
    let d_2 = 2.0 * a / right_edge_length;

    let p_bc = right_edge_length - (d_c * d_c - d_2 * d_2).max(0.0).sqrt();

    if d_2.abs() < f32::EPSILON {
        dbg!("DEGEN 2");
        if p_bc < right_midpoint {
            dbg!("A");
            let q = (p_bc + base_midpoint) / (right_midpoint + base_midpoint);
            return interpolate_velocity_with_offsets(
                q,
                base_velocity,
                right_velocity,
                base_offset,
                from_right_offset,
            );
        }
        dbg!("B");
        let q = (p_bc - right_midpoint) / (right_midpoint + left_midpoint);
        return interpolate_velocity_with_offsets(
            q,
            right_velocity,
            left_velocity,
            from_right_offset,
            from_left_offset,
        );
    }

    let q1: f32;
    let q2: f32;
    let q3: f32;
    let v1: [f32; 2];
    let v2: [f32; 2];
    let v3: [f32; 2];

    dbg!(&angles);

    dbg!(base_velocity);
    dbg!(right_velocity);
    dbg!(left_velocity);

    dbg!(base_offset);
    dbg!(to_right_offset);
    dbg!(from_right_offset);
    dbg!(to_left_offset);
    dbg!(from_left_offset);

    dbg!(mod_tau(base_velocity[1] + base_offset));
    dbg!(mod_tau(right_velocity[1] + from_right_offset));
    dbg!(mod_tau(left_velocity[1] + from_left_offset));

    dbg!(base_midpoint);
    dbg!(right_midpoint);
    dbg!(left_midpoint);

    dbg!(base_edge_length);
    dbg!(right_edge_length);
    dbg!(left_edge_length);

    dbg!(p_ab);
    dbg!(p_bc);
    dbg!(p_ca);

    if p_ab < base_midpoint {
        q1 = (p_ab + left_midpoint) / (base_midpoint + left_midpoint);
        v1 = interpolate_velocity_with_offsets(
            q1,
            left_velocity,
            base_velocity,
            from_left_offset,
            base_offset,
        );
    } else {
        q1 = (p_ab - base_midpoint) / (base_midpoint + right_midpoint);
        v1 = interpolate_velocity_with_offsets(
            q1,
            base_velocity,
            right_velocity,
            base_offset,
            from_right_offset,
        );
    }

    if p_bc < right_midpoint {
        q2 = (p_bc + base_midpoint) / (right_midpoint + base_midpoint);
        v2 = interpolate_velocity_with_offsets(
            q2,
            base_velocity,
            right_velocity,
            base_offset,
            from_right_offset,
        );
    } else {
        q2 = (p_bc - right_midpoint) / (right_midpoint + left_midpoint);
        v2 = interpolate_velocity_with_offsets(
            q2,
            right_velocity,
            left_velocity,
            from_right_offset,
            from_left_offset,
        );
    }

    if p_ca < left_midpoint {
        q3 = (p_ca + right_midpoint) / (left_midpoint + right_midpoint);
        v3 = interpolate_velocity_with_offsets(
            q3,
            right_velocity,
            left_velocity,
            from_right_offset,
            from_left_offset,
        );
    } else {
        q3 = (p_ca - left_midpoint) / (left_midpoint + base_midpoint);
        v3 = interpolate_velocity_with_offsets(
            q3,
            left_velocity,
            base_velocity,
            from_left_offset,
            base_offset,
        );
    }

    let v = [v1, v2, v3];
    let d = [d_1, d_2, d_3];

    dbg!(&[q1, q2, q3]);
    dbg!(&v);
    dbg!(&d);

    let mut vp = [0.0f32, 0.0];
    let mut w_total = 0.0;
    for i in 0..3 {
        w_total += 1.0 / d[i].max(f32::EPSILON);
    }

    dbg!(w_total);

    for i in 0..3 {
        dbg!(&v[i]);
        let mut scaled_v = v[i];
        let normalized_weight = (1.0 / d[i].max(f32::EPSILON)) / w_total;
        dbg!(normalized_weight);
        scaled_v[0] = scaled_v[0] * normalized_weight;
        dbg!(scaled_v);
        vp = add_velocity(vp, scaled_v);
        dbg!(vp);
    }

    vp
}

#[test]
fn test_interior_interpolation_logic() {
    let grid = MeshGrid::new(5);
    let mut velocity = vec![[0.0f32; 2]; grid.edge_cell_adjacency().rows()];
    // These three edges form a triangle
    let base_edge = 349;
    let primary_left_edge = 342;
    let primary_right_edge = 350;
    let secondary_left_edge = 355;
    let secondary_right_edge = 52;

    velocity[base_edge] = [90.79217529, 1.03493309];
    velocity[primary_left_edge] = [91.61075592, 1.95110750];
    velocity[primary_right_edge] = [94.50257111, 6.06997538];
    velocity[secondary_left_edge] = [90.13909912, 2.02728319];
    velocity[secondary_right_edge] = [86.26971436, 2.97428370];

    let edge_velocity = velocity[base_edge];

    let mut edge_lengths = vec![0.0f32; grid.edge_cell_adjacency().rows()];
    for (i, length) in edge_lengths.iter_mut().enumerate() {
        let left_vertex_idx = grid.edge_vertex_adjacency().data()[i * 2] as usize;
        let right_vertex_idx = grid.edge_vertex_adjacency().data()[i * 2 + 1] as usize;
        let left_vertex = grid.points()[left_vertex_idx];
        let right_vertex = grid.points()[right_vertex_idx];
        *length = left_vertex.distance(right_vertex);
    }

    let primary_cell = grid.edge_cell_adjacency().data()[base_edge * 2];
    let secondary_cell = grid.edge_cell_adjacency().data()[base_edge * 2 + 1];
    let mut cell = primary_cell;
    let angle_offset = PI;
    let mut d = edge_lengths[base_edge] / 2.0;
    if mod_tau(edge_velocity[1] + angle_offset) > PI {
        dbg!("SECONDARY CELL");
        cell = secondary_cell;
        d = edge_lengths[base_edge] - d;
    }
    let edges = get_adjacent_edges(
        base_edge as u32,
        cell,
        grid.edge_cell_adjacency().data(),
        grid.edge_vertex_adjacency().data(),
        grid.cell_edge_adjacency().data(),
    );

    let left_edge = edges[0];
    let right_edge = edges[1];

    dbg!(left_edge);
    dbg!(right_edge);

    let angles = compute_angles(
        base_edge as u32,
        left_edge as u32,
        right_edge as u32,
        &edge_lengths,
    );

    let angle = edge_velocity[1];

    let critical_angle = compute_angle_to_apex_vertex(d, base_edge as u32, angles, &edge_lengths);
    let effective_angle = mod_tau(angle + angle_offset);
    let l_and_d = subcell_crossing_distance(
        effective_angle,
        d,
        base_edge as u32,
        cell,
        grid.edge_cell_adjacency().data(),
        grid.edge_vertex_adjacency().data(),
        grid.cell_edge_adjacency().data(),
        &edge_lengths,
    );
    let l_exit = l_and_d[0];
    dbg!(angle);
    dbg!(critical_angle);
    dbg!(effective_angle);
    dbg!(l_exit);

    let remaining_mag = edge_velocity[0] * 1.0 / 60.0;
    dbg!(remaining_mag);

    let departure_position = map_to_reference_frame(
        d,
        base_edge as u32,
        [remaining_mag, mod_tau(angle + angle_offset)],
        &edge_lengths,
    );
    dbg!(&departure_position);

    let interpolated_velocity = interpolate_edge_velocities(
        [1.51320291, 4.17652607],
        angles,
        [base_edge as u32, left_edge as u32, right_edge as u32],
        &edge_lengths,
        &velocity,
        grid.edge_parallel_transport().indptr().as_slice().unwrap(),
        grid.edge_parallel_transport().indices(),
        grid.edge_parallel_transport().data(),
    );

    dbg!(&interpolated_velocity);
    assert_eq!(interpolated_velocity, velocity[base_edge]);
}
