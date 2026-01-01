const PI: f32 = 3.14159265359;
const TAU: f32 = 6.283185230718;
const DT: f32 = 0.01666666667;
const EPS: f32 = 1e-6;

@group(0) @binding(0) var<storage, read> velocity_in: array<vec2<f32>>;
@group(0) @binding(1) var<storage, read_write> velocity_out: array<vec2<f32>>;

@group(1) @binding(0) var<storage, read> edge_vertex_indices: array<u32>;
@group(1) @binding(1) var<storage, read> edge_cell_indices: array<u32>;
@group(1) @binding(2) var<storage, read> cell_edge_indices: array<u32>;
@group(1) @binding(3) var<storage, read> cell_vertices: array<u32>;
@group(1) @binding(4) var<storage, read> vertex_edge_offsets: array<u32>;
@group(1) @binding(5) var<storage, read> vertex_edge_indices: array<u32>;
@group(1) @binding(6) var<storage, read> vertex_angle_offsets: array<f32>;
@group(1) @binding(7) var<storage, read> cell_cell_indices: array<u32>;
@group(1) @binding(8) var<storage, read> vertex_cell_offsets: array<u32>;
@group(1) @binding(9) var<storage, read> vertex_cell_indices: array<u32>;
@group(1) @binding(10) var<storage, read> edge_lengths: array<f32>;

fn mod_tau(theta: f32) -> f32 {
        return (theta + TAU) % TAU;
}

fn add_velocity(vel_a: vec2<f32>, vel_b: vec2<f32>) -> vec2<f32> {
        if vel_a.x < EPS {
                return vel_b;
        }
        if vel_b.x < EPS {
                return vel_a;
        }

        let ax = vel_a.x * cos(vel_a.y);
        let ay = vel_a.x * sin(vel_a.y);
        let bx = vel_b.x * cos(vel_b.y);
        let by = vel_b.x * sin(vel_b.y);

        let rx = ax + bx;
        let ry = ay + by;

        let new_mag = sqrt(rx * rx + ry * ry);
        if new_mag < EPS {
                return vec2<f32>(0.0, 0.0);
        }
        let new_angle = atan2(ry, rx);
        return vec2<f32>(new_mag, mod_tau(new_angle));
}

fn interpolate_velocity(q: f32, vel_a: vec2<f32>, vel_b: vec2<f32>) -> vec2<f32> {
        let va = vec2<f32>((1.0 - q) * vel_a.x, vel_a.y);
        let vb = vec2<f32>(q * vel_b.x, vel_b.y);
        return add_velocity(va, vb);

}

fn interpolate_velocity_with_offsets(q: f32, vel_a: vec2<f32>, vel_b: vec2<f32>, vel_a_offset: f32, vel_b_offset: f32) -> vec2<f32> {
        return interpolate_velocity(q, vec2<f32>(vel_a.x, mod_tau(vel_a_offset + vel_a.y)), vec2<f32>(vel_b.x, mod_tau(vel_b_offset + vel_b.y)));
}

// Computes the angles between three edges. The order of the angles
// is (left_base_angle, right_base_angle, apex_angle)
fn compute_angles(base: u32, left: u32, right: u32) -> vec3<f32> {
        let a = edge_lengths[base];
        let b = edge_lengths[left];
        let c = edge_lengths[right];

        let a_squared = a * a;
        let b_squared = b * b;
        let c_squared = c * c;

        let left_base_angle = acos(
                clamp((a_squared + b_squared - c_squared) /
                (2 * a * b), -1.0, 1.0));
        let right_base_angle = acos(clamp((a_squared + c_squared - b_squared) / (2 * a * c), -1.0, 1.0));
        let apex_angle = acos(clamp((b_squared + c_squared - a_squared) / (2 * b * c), -1.0, 1.0));

        return vec3<f32>(left_base_angle, right_base_angle, apex_angle);
}

// Computes the angle to the apex vertex from a point at distance `d` from the left vertex on the base edge.
fn compute_angle_to_apex_vertex(d: f32, base_edge: u32, angles: vec3<f32>) -> f32 {
        let base_edge_length = edge_lengths[base_edge];
        return atan2(sin(angles.x) * sin(angles.y), (d / base_edge_length) * sin(angles.x + angles.y) - sin(angles.y) * cos(angles.x));
}

// Gets the adjacent edges to this one in the indicated cell.
// Returns (left_edge, right_edge)
fn get_adjacent_edges(edge: u32, cell: u32) -> vec2<u32> {
        var left_edge = edge;
        var right_edge = edge;

        let left_vertex = edge_vertex_indices[edge * 2u];
        let right_vertex = edge_vertex_indices[edge * 2u + 1u];

        for (var i = 0u; i < 3u; i++) {
                let other_edge = cell_edge_indices[cell * 3u + i];
                if other_edge == edge {
                        continue;
                }
                let other_left_vertex = edge_vertex_indices[other_edge * 2u];
                let other_right_vertex = edge_vertex_indices[other_edge * 2u + 1u];
                if other_left_vertex == left_vertex || other_right_vertex == left_vertex {
                        left_edge = other_edge;
                } else if other_left_vertex == right_vertex || other_right_vertex == right_vertex {
                        right_edge = other_edge;
                }
        }

        return vec2<u32>(left_edge, right_edge);
}

// returns `l_exit` in `vec.x` and `d_exit` in `vec.y`. `l_exit` is the crossing distance and `d_exit` is the distance to the shared vertex along the exit edge.
fn subcell_crossing_distance(theta: f32, d: f32, base_edge: u32, cell: u32) -> vec2<f32> {
        let adjacent_edges = get_adjacent_edges(base_edge, cell);
        let left_edge= adjacent_edges.x;
        let right_edge = adjacent_edges.y;

        let base_edge_length = edge_lengths[base_edge];
        let left_edge_length = edge_lengths[left_edge];
        let right_edge_length = edge_lengths[right_edge];

        let angles = compute_angles(base_edge, left_edge, right_edge);
        let critical_angle = compute_angle_to_apex_vertex(d, base_edge, angles);
        let left_base_angle = angles.x;
        let right_base_angle = angles.y;

        var l_exit = 0.0;
        var d_exit = 0.0;
        if theta > EPS && theta <= critical_angle {
                let denom = sin(theta + left_base_angle);
                if abs(denom) < EPS || abs(sin(left_base_angle)) < EPS {
                        return vec2<f32>(0.0, 0.0);
                }
                l_exit = d * sin(left_base_angle) / denom;
                d_exit = l_exit * sin(theta) / sin(left_base_angle);
        } else if theta > EPS && theta < PI {
                let denom = sin(theta - right_base_angle);
                if abs(denom) < EPS || abs(sin(right_base_angle)) < EPS {
                        return vec2<f32>(0.0, 0.0);
                }
                l_exit = (base_edge_length - d) * sin(right_base_angle) / denom;
                d_exit = l_exit * sin(theta) / sin(right_base_angle);
        } else if abs(theta) <= EPS {
                l_exit = d;
                d_exit = 0.0;
        } else if abs(theta - PI) <= EPS {
                l_exit = base_edge_length - d;
                d_exit = 0.0;
        }

        return vec2<f32>(l_exit, d_exit);
}

// maps a polar position from point arbitrary point d along the base edge to the same position described
// by d = midpoint.
fn map_to_reference_frame(d: f32, edge: u32, velocity: vec2<f32>) -> vec2<f32> {
        let base_edge_length = edge_lengths[edge];
        let midpoint = base_edge_length / 2.0;

        let midpoint_offset = midpoint - d;
        let x = midpoint_offset + velocity.x * cos(velocity.y);
        let y = velocity.x * sin(velocity.y);

        let mag = sqrt(x * x + y * y);
        if mag < EPS {
                return vec2<f32>(0.0, 0.0);
        }

        let theta = mod_tau(atan2(y, x));
        return vec2<f32>(mag, theta);
}

fn interpolate_edge_velocities(pos: vec2<f32>, angles: vec3<f32>, edges: vec3<u32>, cell: u32) -> vec2<f32> {
        let base_edge_length = edge_lengths[edges.x];
        let left_edge_length = edge_lengths[edges.y];
        let right_edge_length = edge_lengths[edges.z];

        let base_midpoint = base_edge_length / 2.0;
        let left_midpoint = left_edge_length / 2.0;
        let right_midpoint = right_edge_length / 2.0;

        let base_velocity = velocity_in[edges.x];
        let left_velocity = velocity_in[edges.y];
        let right_velocity = velocity_in[edges.z];

        let base_primary_cell = edge_cell_indices[edges.x * 2u];
        let base_secondary_cell = edge_cell_indices[edges.x * 2u + 1u];

        var base_offset = 0.0;
        if base_secondary_cell == cell {
                base_offset = mod_tau(base_offset + PI);
        }

        let left_primary_cell = edge_cell_indices[edges.y * 2u];
        let left_secondary_cell = edge_cell_indices[edges.y * 2u + 1u];
        var left_offset = angles.x;
        if left_primary_cell == cell {
                left_offset = mod_tau(left_offset + PI);
        }

        let right_primary_cell = edge_cell_indices[edges.z * 2u];
        let right_secondary_cell = edge_cell_indices[edges.z * 2u + 1u];

        var right_offset = mod_tau(-angles.y);
        if right_primary_cell == cell {
                right_offset = mod_tau(right_offset + PI);
        }

        // The projection of `pos` to the base edge.
        let p_ab = base_midpoint - pos.x * cos(pos.y);

        let d_1 = pos.x * sin(pos.y);
        if d_1 < EPS {
                // Degenerate case, point is along base edge
                if p_ab < base_midpoint {
                        let q = (p_ab + left_midpoint) / (base_midpoint + left_midpoint);
                        return interpolate_velocity_with_offsets(q, left_velocity, base_velocity, left_offset, base_offset);
                } else {
                        let q = (p_ab - base_midpoint) / (base_midpoint + right_midpoint);
                        return interpolate_velocity_with_offsets(q, base_velocity, right_velocity, base_offset, right_offset);
                }
        }
        let d_a = sqrt(p_ab * p_ab + d_1 * d_1);
        let phi_a = acos(clamp((d_1 * d_1 + d_a * d_a - p_ab * p_ab) / (2 * d_1 * d_a), -1.0, 1.0));
        let phi_b = TAU / 4.0 - phi_a;
        let phi_c = angles.x - phi_b;

        // The projection of `pos` to the left edge
        let p_ca = left_edge_length - d_a * cos(phi_c);

        let d_3 = d_a * sin(phi_c);
        if d_3 < EPS {
                // Degenerate case, point is along the left edge
                if p_ca < left_midpoint {
                        let q = (p_ca + right_midpoint) / (left_midpoint + right_midpoint);
                        return interpolate_velocity_with_offsets(q, right_velocity, left_velocity, right_offset, left_offset);
                } else {
                        let q = (p_ca - left_midpoint) / (left_midpoint + base_midpoint);
                        return interpolate_velocity_with_offsets(q, left_velocity, base_velocity, left_offset, base_offset);
                }
        }
        let d_c = sqrt(p_ca * p_ca + d_3 * d_3);
        let d_b = sqrt((base_edge_length - p_ab) * (base_edge_length - p_ab) + d_1 * d_1);
        let s = (d_b + d_c + right_edge_length) / 2.0;
        let a = sqrt(max(s * (s - d_b) * (s - d_c) * (s - right_edge_length), 0.0));
        let d_2 = 2 * a / right_edge_length;

        // The projection of `pos` to the right edge
        let p_bc = right_edge_length - sqrt(max(d_c * d_c - d_2 * d_2, 0.0));

        if d_2 < EPS {
                // Degenerate case, point is along the right edge
                if p_bc < right_midpoint {
                        let q = (p_bc + base_midpoint) / (right_midpoint + base_midpoint);
                        return interpolate_velocity_with_offsets(q, base_velocity, right_velocity, base_offset, right_offset);
                } else {
                        let q = (p_bc - right_midpoint) / (right_midpoint + left_midpoint);
                        return interpolate_velocity_with_offsets(q, right_velocity, left_velocity, right_offset, left_offset);
                }
        }

        var q1: f32;
        var q2: f32;
        var q3: f32;
        var v1: vec2<f32>;
        var v2: vec2<f32>;
        var v3: vec2<f32>;

        if p_ab < base_midpoint {
                q1 = (p_ab + left_midpoint) / (base_midpoint + left_midpoint);
                v1 = interpolate_velocity_with_offsets(q1, left_velocity, base_velocity, left_offset, base_offset);
        } else {
                q1 = (p_ab - base_midpoint) / (base_midpoint + right_midpoint);
                v1 = interpolate_velocity_with_offsets(q1, base_velocity, right_velocity, base_offset, right_offset);
        }

        if p_bc < right_midpoint {
                q2 = (p_bc + base_midpoint) / (right_midpoint + base_midpoint);
                v2 = interpolate_velocity_with_offsets(q2, base_velocity, right_velocity, base_offset, right_offset);
        } else {
                q2 = (p_bc - right_midpoint) / (right_midpoint + left_midpoint);
                v2 = interpolate_velocity_with_offsets(q2, right_velocity, left_velocity, right_offset, left_offset);
        }

        if p_ca < left_midpoint {
                q3 = (p_ca + right_midpoint) / (left_midpoint + right_midpoint);
                v3 = interpolate_velocity_with_offsets(q3, right_velocity, left_velocity, right_offset, left_offset);
        } else {
                q3 = (p_ca - left_midpoint) / (left_midpoint + base_midpoint);
                v3 = interpolate_velocity_with_offsets(q3, left_velocity, base_velocity, left_offset, base_offset);
        }

        let v = array<vec2<f32>, 3>(v1, v2, v3);
        let d = array<f32, 3>(d_1, d_2, d_3);

        var vp= vec2<f32>(0.0, 0.0);
        var d_total = 0.0;
        for (var i = 0u; i < 3u; i++) {
                d_total += 1.0 / (d[i] + EPS);
        }
        for (var i = 0u; i < 3u; i++) {
                var scaled_v = v[i];
                scaled_v.x = scaled_v.x / (d[i] + EPS);
                vp = add_velocity(vp, scaled_v);
        }
        vp.x = vp.x / d_total;

        return vp;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
        let edge_idx = global_id.x;
        let num_edges = arrayLength(&velocity_in);

        if (edge_idx >= num_edges) {
                return;
        }

        let edge_velocity = velocity_in[edge_idx];
        if abs(edge_velocity.x) < EPS {
                return;
        }

        var base_edge = edge_idx;
        var remaining_mag = edge_velocity.x * DT;
        // trace backward, flip the current angle
        var angle = mod_tau(edge_velocity.y + PI);
        var angle_offset = 0.0;
        var d = edge_lengths[base_edge] / 2.0;
        var l_exit = 0.0;
        var cell: u32;
        var angles: vec3<f32>;
        var adjacent_edges: vec2<u32>;
        var left_edge: u32;
        var right_edge: u32;

        while (remaining_mag > EPS) {
                let primary_cell = edge_cell_indices[base_edge * 2u];
                let secondary_cell = edge_cell_indices[base_edge * 2u + 1u];
                cell = primary_cell;
                if mod_tau(angle + angle_offset) > PI {
                        angle_offset = mod_tau(angle_offset + PI);
                        cell = secondary_cell;
                        d = edge_lengths[base_edge] - d;
                }
                adjacent_edges = get_adjacent_edges(base_edge, cell);
                left_edge = adjacent_edges.x;
                right_edge = adjacent_edges.y;
                angles = compute_angles(base_edge, left_edge, right_edge);
                let critical_angle = compute_angle_to_apex_vertex(d, base_edge, angles);
                let effective_angle = mod_tau(angle + angle_offset);
                let l_and_d = subcell_crossing_distance(effective_angle, d, base_edge, cell);
                l_exit = l_and_d.x;
                if l_exit < EPS || remaining_mag <= l_exit {
                        break;
                }
                d = l_and_d.y;
                remaining_mag = max(remaining_mag - l_exit, 0.0);
                if effective_angle > 0.0 && effective_angle < critical_angle {
                        base_edge = left_edge;
                        angle_offset = mod_tau(angle_offset + angles.x);
                } else if effective_angle > 0.0 && effective_angle < PI {
                        base_edge = right_edge;
                        angle_offset = mod_tau(angle_offset - angles.y);
                }
        }

        let departure_position = map_to_reference_frame(d, base_edge, vec2<f32>(remaining_mag, mod_tau(angle + angle_offset)));
        var interpolated_velocity = interpolate_edge_velocities(departure_position, angles, vec3<u32>(base_edge, left_edge, right_edge), cell);
        interpolated_velocity.y = mod_tau(interpolated_velocity.y + angle_offset);
        let delta_v = add_velocity(interpolated_velocity, vec2<f32>(edge_velocity.x, mod_tau(edge_velocity.y + PI)));
        // update velocity out with the advection contribution
        velocity_out[edge_idx] = add_velocity(velocity_out[edge_idx], delta_v);
}
