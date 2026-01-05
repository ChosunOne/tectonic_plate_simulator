use std::f32::consts::{PI, TAU};

use tectonic_plate_simulator::{
    LocalFrame, constants::SPHERE_RADIUS, resources::mantle_grid::MantleGrid,
};

fn shader_mod_tau(theta: f32) -> f32 {
    (theta + TAU) % TAU
}

fn shader_add_velocity(vel_a: [f32; 2], vel_b: [f32; 2]) -> [f32; 2] {
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
    [mag, shader_mod_tau(angle)]
}

fn shader_interpolate_velocity(q: f32, vel_a: [f32; 2], vel_b: [f32; 2]) -> [f32; 2] {
    if q.abs() < f32::EPSILON {
        return vel_a;
    }
    if (1.0 - q).abs() < f32::EPSILON {
        return vel_b;
    }
    let va = [(1.0 - q) * vel_a[0], vel_a[1]];
    let vb = [q * vel_b[0], vel_b[1]];
    shader_add_velocity(va, vb)
}

fn shader_interpolate_velocity_with_offsets(
    q: f32,
    vel_a: [f32; 2],
    vel_b: [f32; 2],
    vel_a_offset: f32,
    vel_b_offset: f32,
) -> [f32; 2] {
    shader_interpolate_velocity(
        q,
        [vel_a[0], shader_mod_tau(vel_a_offset + vel_a[1])],
        [vel_b[0], shader_mod_tau(vel_b_offset + vel_b[1])],
    )
}

fn shader_compute_angles(base: u32, left: u32, right: u32, edge_lengths: &[f32]) -> [f32; 3] {
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

fn shader_compute_angle_to_apex_vertex(
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

fn shader_get_adjacent_edges(
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

fn shader_subcell_crossing_distance(
    theta: f32,
    d: f32,
    base_edge: u32,
    cell: u32,
    edge_cell_indices: &[u32],
    edge_vertex_indices: &[u32],
    cell_edge_indices: &[u32],
    edge_lengths: &[f32],
) -> [f32; 2] {
    let adjacent_edges = shader_get_adjacent_edges(
        base_edge,
        cell,
        edge_cell_indices,
        edge_vertex_indices,
        cell_edge_indices,
    );

    let left_edge = adjacent_edges[0];
    let right_edge = adjacent_edges[1];

    let base_edge_length = edge_lengths[base_edge as usize];

    let angles = shader_compute_angles(base_edge, left_edge, right_edge, edge_lengths);
    let critical_angle = shader_compute_angle_to_apex_vertex(d, base_edge, angles, edge_lengths);
    let left_base_angle = angles[0];
    let right_base_angle = angles[1];

    let mut l_exit = 0.0;
    let mut d_exit = 0.0;

    if theta > f32::EPSILON && theta <= critical_angle {
        let denom = (theta + left_base_angle).sin();
        if denom.abs() < f32::EPSILON || left_base_angle.sin().abs() < f32::EPSILON {
            return [0.0, 0.0];
        }
        l_exit = d * left_base_angle.sin() / denom;
        d_exit = l_exit * theta.sin() / left_base_angle.sin();
    } else if theta > f32::EPSILON && theta < PI {
        let denom = (theta - right_base_angle).sin();
        if denom.abs() < f32::EPSILON || right_base_angle.sin().abs() < f32::EPSILON {
            return [0.0, 0.0];
        }
        l_exit = (base_edge_length - d) * right_base_angle.sin() / denom;
        d_exit = l_exit * theta.sin() / right_base_angle.sin();
    } else if theta.abs() < f32::EPSILON {
        l_exit = d;
        d_exit = 0.0;
    } else if (theta - PI) <= f32::EPSILON {
        l_exit = base_edge_length - d;
        d_exit = 0.0;
    }

    [l_exit, d_exit]
}

fn shader_map_to_reference_frame(
    d: f32,
    edge: u32,
    velocity: [f32; 2],
    edge_lengths: &[f32],
) -> [f32; 2] {
    let base_edge_length = edge_lengths[edge as usize];
    let midpoint = base_edge_length / 2.0;

    let midpoint_offset = midpoint - d;
    let x = midpoint_offset + velocity[0] * velocity[1].cos();
    let y = velocity[0] * velocity[1].sin();

    let mag = (x * x + y * y).sqrt();
    if mag < f32::EPSILON {
        return [0.0, 0.0];
    }

    let theta = shader_mod_tau(y.atan2(x));
    [mag, theta]
}

fn shader_interpolate_edge_velocities(
    pos: [f32; 2],
    angles: [f32; 3],
    edges: [u32; 3],
    cell: u32,
    edge_lengths: &[f32],
    velocity_in: &[[f32; 2]],
    edge_cell_indices: &[u32],
) -> [f32; 2] {
    let base_edge_length = edge_lengths[edges[0] as usize];
    let left_edge_length = edge_lengths[edges[1] as usize];
    let right_edge_length = edge_lengths[edges[2] as usize];

    let base_midpoint = base_edge_length / 2.0;
    let left_midpoint = left_edge_length / 2.0;
    let right_midpoint = right_edge_length / 2.0;

    let base_velocity = velocity_in[edges[0] as usize];
    let left_velocity = velocity_in[edges[1] as usize];
    let right_velocity = velocity_in[edges[2] as usize];

    let base_secondary_cell = edge_cell_indices[edges[0] as usize * 2 + 1];

    let mut base_offset = 0.0;
    if base_secondary_cell == cell {
        base_offset = shader_mod_tau(base_offset - PI);
    }

    let left_primary_cell = edge_cell_indices[edges[1] as usize * 2];
    let mut to_left_offset = angles[0];
    if left_primary_cell == cell {
        to_left_offset = shader_mod_tau(to_left_offset + PI);
    }
    let from_left_offset = shader_mod_tau(-to_left_offset);

    let right_primary_cell = edge_cell_indices[edges[2] as usize * 2];
    let mut to_right_offset = shader_mod_tau(-angles[1]);
    if right_primary_cell == cell {
        to_right_offset = shader_mod_tau(to_right_offset + PI);
    }
    let from_right_offset = shader_mod_tau(-to_right_offset);

    let p_ab = base_midpoint - pos[0] * pos[1].cos();

    let d_1 = pos[0] * pos[1].sin();
    if d_1 < f32::EPSILON {
        if p_ab < base_midpoint {
            let q = (p_ab + left_midpoint) / (base_midpoint + left_midpoint);
            return shader_interpolate_velocity_with_offsets(
                q,
                left_velocity,
                base_velocity,
                from_left_offset,
                base_offset,
            );
        }
        let q = (p_ab - base_midpoint) / (base_midpoint + right_midpoint);
        return shader_interpolate_velocity_with_offsets(
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
    if d_3 < f32::EPSILON {
        if p_ca < left_midpoint {
            let q = (p_ca + right_midpoint) / (left_midpoint + right_midpoint);
            return shader_interpolate_velocity_with_offsets(
                q,
                right_velocity,
                left_velocity,
                from_right_offset,
                from_left_offset,
            );
        }
        let q = (p_ca - left_midpoint) / (left_midpoint + base_midpoint);
        return shader_interpolate_velocity_with_offsets(
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

    if d_2 < f32::EPSILON {
        if p_bc < right_midpoint {
            let q = (p_bc + base_midpoint) / (right_midpoint + base_midpoint);
            return shader_interpolate_velocity_with_offsets(
                q,
                base_velocity,
                right_velocity,
                base_offset,
                from_right_offset,
            );
        }
        let q = (p_bc - right_midpoint) / (right_midpoint + left_midpoint);
        return shader_interpolate_velocity_with_offsets(
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
    dbg!(to_left_offset);

    dbg!(shader_mod_tau(base_velocity[1] + base_offset));
    dbg!(shader_mod_tau(right_velocity[1] + from_right_offset));
    dbg!(shader_mod_tau(left_velocity[1] + from_left_offset));

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
        v1 = shader_interpolate_velocity_with_offsets(
            q1,
            left_velocity,
            base_velocity,
            from_left_offset,
            base_offset,
        );
    } else {
        q1 = (p_ab - base_midpoint) / (base_midpoint + right_midpoint);
        v1 = shader_interpolate_velocity_with_offsets(
            q1,
            base_velocity,
            right_velocity,
            base_offset,
            from_right_offset,
        );
    }

    if p_bc < right_midpoint {
        q2 = (p_bc + base_midpoint) / (right_midpoint + base_midpoint);
        v2 = shader_interpolate_velocity_with_offsets(
            q2,
            base_velocity,
            right_velocity,
            base_offset,
            from_right_offset,
        );
    } else {
        q2 = (p_bc - right_midpoint) / (right_midpoint + left_midpoint);
        v2 = shader_interpolate_velocity_with_offsets(
            q2,
            right_velocity,
            left_velocity,
            from_right_offset,
            from_left_offset,
        );
    }

    if p_ca < left_midpoint {
        q3 = (p_ca + right_midpoint) / (left_midpoint + right_midpoint);
        v3 = shader_interpolate_velocity_with_offsets(
            q3,
            right_velocity,
            left_velocity,
            from_right_offset,
            from_left_offset,
        );
    } else {
        q3 = (p_ca - left_midpoint) / (left_midpoint + base_midpoint);
        v3 = shader_interpolate_velocity_with_offsets(
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
    let mut d_total = 0.0;
    for i in 0..3 {
        d_total += 1.0 / d[i].max(f32::EPSILON);
    }

    dbg!(d_total);

    for i in 0..3 {
        let mut scaled_v = v[i];
        dbg!((1.0 / d[i].max(f32::EPSILON)) / d_total);
        scaled_v[0] = scaled_v[0] * ((1.0 / d[i].max(f32::EPSILON)) / d_total);
        dbg!(scaled_v);
        vp = shader_add_velocity(vp, scaled_v);
        dbg!(vp[0]);
    }

    vp
}

#[test]
fn test_interior_interpolation_logic() {
    let grid = MantleGrid::new(20);
    let mut velocity = vec![[0.0f32; 2]; grid.edge_cell_adjacency().len()];
    // These three edges form a triangle
    let base_edge = 233;
    let left_edge = 229;
    let right_edge = 10;
    let base_frame = LocalFrame::from_edge(&grid, base_edge);
    let left_frame = LocalFrame::from_edge(&grid, left_edge);
    let right_frame = LocalFrame::from_edge(&grid, right_edge);

    velocity[base_edge] = [1000.0, base_frame.bearing_to_local_angle(PI / 2.0)];
    velocity[left_edge] = [1000.0, left_frame.bearing_to_local_angle(PI / 2.0)];
    velocity[right_edge] = [1000.0, right_frame.bearing_to_local_angle(PI / 2.0)];

    let mut edge_lengths = vec![0.0f32; grid.edge_cell_adjacency().len()];
    for (i, length) in edge_lengths.iter_mut().enumerate() {
        let left_vertex_idx = grid.edge_vertex_adjacency().indices()[i * 2] as usize;
        let right_vertex_idx = grid.edge_vertex_adjacency().indices()[i * 2 + 1] as usize;
        let left_vertex = grid.sphere().raw_points()[left_vertex_idx] * SPHERE_RADIUS;
        let right_vertex = grid.sphere().raw_points()[right_vertex_idx] * SPHERE_RADIUS;
        *length = left_vertex.distance(right_vertex);
    }

    let angles = shader_compute_angles(
        base_edge as u32,
        left_edge as u32,
        right_edge as u32,
        &edge_lengths,
    );

    let cell = grid.edge_cell_adjacency().indices()[base_edge * 2];

    let interpolated_velocity = shader_interpolate_edge_velocities(
        [16.6667, 1.5708],
        angles,
        [base_edge as u32, left_edge as u32, right_edge as u32],
        cell,
        &edge_lengths,
        &velocity,
        grid.edge_cell_adjacency().indices(),
    );

    assert_eq!(interpolated_velocity, velocity[base_edge]);
}
