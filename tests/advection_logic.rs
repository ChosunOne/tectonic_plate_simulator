use std::f32::consts::{PI, TAU};

use tectonic_plate_simulator::{
    LocalFrame, constants::SPHERE_RADIUS, plugins::departure_info, resources::mesh_grid::MeshGrid,
};

fn shader_mod_tau(theta: f32) -> f32 {
    if theta >= 0.0 && theta < TAU {
        return theta;
    }
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

    let base_secondary_cell = edge_cell_indices[edges[0] as usize * 2 + 1];

    let mut base_offset = 0.0;
    if base_secondary_cell == cell {
        dbg!("SHADER SECONDARY");
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

    let p_ab = base_midpoint - pos_x * pos_y.cos();

    let d_1 = pos_x * pos_y.sin();
    dbg!(d_1);
    if d_1.abs() < f32::EPSILON {
        dbg!("DEGEN 1");
        if p_ab < base_midpoint {
            dbg!("A");
            let q = (p_ab + left_midpoint) / (base_midpoint + left_midpoint);
            let mut v = shader_interpolate_velocity_with_offsets(
                q,
                left_velocity,
                base_velocity,
                from_left_offset,
                base_offset,
            );
            if mirror_result {
                v[1] = shader_mod_tau(PI + v[1]);
            }
            return v;
        }
        dbg!("B");
        let q = (p_ab - base_midpoint) / (base_midpoint + right_midpoint);
        dbg!(q);
        dbg!(base_offset);
        dbg!(from_right_offset);
        let mut v = shader_interpolate_velocity_with_offsets(
            q,
            base_velocity,
            right_velocity,
            base_offset,
            from_right_offset,
        );
        if mirror_result {
            dbg!(v[1]);
            dbg!("MIRROR APPLIED");
            v[1] = shader_mod_tau(PI + v[1]);
        }
        return v;
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
            let mut v = shader_interpolate_velocity_with_offsets(
                q,
                right_velocity,
                left_velocity,
                from_right_offset,
                from_left_offset,
            );
            if mirror_result {
                v[1] = shader_mod_tau(PI + v[1]);
            }
            return v;
        }
        let q = (p_ca - left_midpoint) / (left_midpoint + base_midpoint);
        let mut v = shader_interpolate_velocity_with_offsets(
            q,
            left_velocity,
            base_velocity,
            from_left_offset,
            base_offset,
        );
        if mirror_result {
            v[1] = shader_mod_tau(PI + v[1]);
        }
        return v;
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
            let mut v = shader_interpolate_velocity_with_offsets(
                q,
                base_velocity,
                right_velocity,
                base_offset,
                from_right_offset,
            );
            if mirror_result {
                v[1] = shader_mod_tau(PI + v[1]);
            }
            return v;
        }
        dbg!("B");
        let q = (p_bc - right_midpoint) / (right_midpoint + left_midpoint);
        let mut v = shader_interpolate_velocity_with_offsets(
            q,
            right_velocity,
            left_velocity,
            from_right_offset,
            from_left_offset,
        );
        if mirror_result {
            v[1] = shader_mod_tau(PI + v[1]);
        }

        return v;
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

    if mirror_result {
        vp[1] = shader_mod_tau(PI + vp[1]);
    }

    vp
}

#[test]
fn test_interior_interpolation_logic() {
    let grid = MeshGrid::new(100);
    let mut velocity = vec![[0.0f32; 2]; grid.edge_cell_adjacency().len()];
    // These three edges form a triangle
    let base_edge = 2785;
    let primary_left_edge = 2784;
    let primary_right_edge = 2786;
    let secondary_left_edge = 2793;
    let secondary_right_edge = 1940;
    let base_frame = LocalFrame::from_edge(&grid, base_edge);
    let primary_left_frame = LocalFrame::from_edge(&grid, primary_left_edge);
    let primary_right_frame = LocalFrame::from_edge(&grid, primary_right_edge);

    velocity[base_edge] = [17.09977341, 6.28318501];
    velocity[primary_left_edge] = [16.71239471, 0.92208433];
    velocity[primary_right_edge] = [16.72800827, 5.33526325];
    velocity[secondary_left_edge] = [13.33704472, 1.00216007];
    velocity[secondary_right_edge] = [15.19938850, 5.30937481];

    let edge_velocity = velocity[base_edge];

    let mut edge_lengths = vec![0.0f32; grid.edge_cell_adjacency().len()];
    for (i, length) in edge_lengths.iter_mut().enumerate() {
        let left_vertex_idx = grid.edge_vertex_adjacency().indices()[i * 2] as usize;
        let right_vertex_idx = grid.edge_vertex_adjacency().indices()[i * 2 + 1] as usize;
        let left_vertex = grid.sphere().raw_points()[left_vertex_idx] * SPHERE_RADIUS;
        let right_vertex = grid.sphere().raw_points()[right_vertex_idx] * SPHERE_RADIUS;
        *length = left_vertex.distance(right_vertex);
    }

    let primary_cell = grid.edge_cell_adjacency().indices()[base_edge * 2];
    let secondary_cell = grid.edge_cell_adjacency().indices()[base_edge * 2 + 1];
    let mut cell = primary_cell;
    let mut angle_offset = PI;
    let mut d = edge_lengths[base_edge] / 2.0;
    if shader_mod_tau(edge_velocity[1] + angle_offset) > PI {
        dbg!("SECONDARY CELL");
        cell = secondary_cell;
        d = edge_lengths[base_edge] - d;
    }
    let edges = shader_get_adjacent_edges(
        base_edge as u32,
        cell,
        grid.edge_cell_adjacency().indices(),
        grid.edge_vertex_adjacency().indices(),
        grid.cell_edge_adjacency().indices(),
    );

    let left_edge = edges[0];
    let right_edge = edges[1];

    dbg!(left_edge);
    dbg!(right_edge);

    let angles = shader_compute_angles(
        base_edge as u32,
        left_edge as u32,
        right_edge as u32,
        &edge_lengths,
    );

    let angle = edge_velocity[1];

    let critical_angle =
        shader_compute_angle_to_apex_vertex(d, base_edge as u32, angles, &edge_lengths);
    let effective_angle = shader_mod_tau(angle + angle_offset);
    let l_and_d = shader_subcell_crossing_distance(
        effective_angle,
        d,
        base_edge as u32,
        cell,
        grid.edge_cell_adjacency().indices(),
        grid.edge_vertex_adjacency().indices(),
        grid.cell_edge_adjacency().indices(),
        &edge_lengths,
    );
    let l_exit = l_and_d[0];
    dbg!(angle);
    dbg!(critical_angle);
    dbg!(effective_angle);
    dbg!(l_exit);

    let remaining_mag = edge_velocity[0] * 1.0 / 60.0;
    dbg!(remaining_mag);

    let departure_position = shader_map_to_reference_frame(
        d,
        base_edge as u32,
        [remaining_mag, shader_mod_tau(angle + angle_offset)],
        &edge_lengths,
    );
    dbg!(&departure_position);

    let interpolated_velocity = shader_interpolate_edge_velocities(
        [0.28499624, 3.14159274],
        // departure_position,
        angles,
        [base_edge as u32, left_edge as u32, right_edge as u32],
        cell,
        &edge_lengths,
        &velocity,
        grid.edge_cell_adjacency().indices(),
    );

    dbg!(&interpolated_velocity);
    // assert_eq!(interpolated_velocity, velocity[base_edge]);
}
