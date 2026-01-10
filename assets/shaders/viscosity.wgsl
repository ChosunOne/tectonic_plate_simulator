const PI: f32 = 3.1415927;
const TAU: f32 = 6.2831855;
const VISCOSITY: f32 = 0.000;
const EPS: f32 = 1.1920929e-7;

struct SimParams { dt: f32 }

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
@group(2) @binding(11) var<storage, read> edge_centroid_distance: array<f32>;

@group(2) @binding(0) var<uniform> sim_params: SimParams; 

fn mod_tau(theta: f32) -> f32 {
        if theta >= 0.0 && theta < TAU {
                return theta;
        }
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
        let new_angle = mod_tau(atan2(ry, rx));
        return vec2<f32>(new_mag, new_angle);
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

fn get_angle_offset(edge_a_idx: u32, edge_b_idx: u32, primary: bool) -> f32 {
        var cell = edge_cell_indices[edge_a_idx * 2u];
        if !primary {
                cell = edge_cell_indices[edge_a_idx * 2u + 1u];
        }

        let adjacent_edges = get_adjacent_edges(edge_a_idx, cell);

        var left_edge = adjacent_edges.x;
        var right_edge = adjacent_edges.y;

        // swap the left and right edges
        if !primary {
                left_edge = left_edge ^ right_edge;
                right_edge = left_edge ^ right_edge;
                left_edge = left_edge ^ right_edge;
        }
        let angles = compute_angles(edge_a_idx, left_edge, right_edge);

        var angle_offset = 0.0;
        // if the edge's primary cell is the same as our primary cell, its reference is pointing mostly downward, so we need to flip it 180 degrees.

        if edge_cell_indices[edge_b_idx * 2u] == edge_cell_indices[edge_a_idx * 2u] && primary {
                angle_offset += PI;
        }
        if edge_cell_indices[edge_b_idx * 2u] == edge_cell_indices[edge_a_idx * 2u + 1u] && !primary {
                angle_offset += PI;
        }


        // This represents the "left" edge of our triangle
        var angle_sign = 1.0;
        // This is the "right" edge of our triangle
        if right_edge == edge_b_idx {
                angle_offset = mod_tau(angle_offset - angles.y);
                return angle_offset;
        }

        angle_offset = mod_tau(angle_offset + angles.x);
        return angle_offset;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
        // "our" edge
        let edge_idx = global_id.x;
        let num_edges = arrayLength(&velocity_in);

        if (edge_idx >= num_edges) {
                return;
        }

        if VISCOSITY == 0.0 {
                velocity_out[edge_idx] = velocity_in[edge_idx];
                return;
        }

        let primary_cell = edge_cell_indices[edge_idx * 2u];
        let secondary_cell = edge_cell_indices[edge_idx * 2u + 1u];

        var avg_vel = vec2<f32>(0.0, 0.0);

        for (var i: u32 = 0u; i < 3; i++) {
                let primary_edge_idx = cell_edge_indices[primary_cell * 3u + i];
                // only average the *other* velocities surrounding the one we want to update.
                if primary_edge_idx == edge_idx {
                        continue;
                }

                let angle_offset = get_angle_offset(edge_idx, primary_edge_idx, true);

                let primary_edge_velocity = velocity_in[primary_edge_idx];
                let adjusted_velocity = vec2<f32>(primary_edge_velocity.x, mod_tau(primary_edge_velocity.y + angle_offset));

                avg_vel = add_velocity(avg_vel, adjusted_velocity);
        }

        for (var i: u32 = 0; i < 3; i++) {
                let secondary_edge_idx = cell_edge_indices[secondary_cell * 3u + i];
                if secondary_edge_idx == edge_idx {
                        continue;
                }

                // Flip 180 since we are dealing with the secondary cell
                let angle_offset = get_angle_offset(edge_idx, secondary_edge_idx, false) + PI;

                let secondary_edge_velocity = velocity_in[secondary_edge_idx];
                let adjusted_velocity = vec2<f32>(secondary_edge_velocity.x, mod_tau(secondary_edge_velocity.y + angle_offset));

                avg_vel = add_velocity(avg_vel, adjusted_velocity);
        }
        avg_vel.x *= 0.25;

        let neg_velocity = vec2<f32>(velocity_in[edge_idx].x, velocity_in[edge_idx].y + PI);
        var avg_diff = add_velocity(avg_vel, neg_velocity);
        avg_diff.x *= sim_params.dt * VISCOSITY;
        if abs(avg_diff.x) < EPS {
                velocity_out[edge_idx] = velocity_in[edge_idx];
                return;
        }
        velocity_out[edge_idx] = add_velocity(velocity_in[edge_idx], avg_diff);
}
