const PI: f32 = 3.14159265359;
const TAU: f32 = 6.283185230718;
const VISCOSITY: f32 = 0.5;
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

fn add_velocity(vel_a: vec2<f32>, vel_b: vec2<f32>) -> vec2<f32> {
        if vel_a.x < EPS {
                return vel_b;
        }
        if vel_b.x < EPS {
                return vel_a;
        }
        let new_mag = sqrt((vel_a.x * vel_a.x + vel_b.x * vel_b.x + 2 * vel_a.x * vel_b.x * cos(vel_b.y - vel_a.y)));
        if new_mag < EPS {
                return vec2<f32>(0.0, 0.0);
        }
        var new_angle = (vel_a.y + atan2(vel_b.x * sin(vel_b.y - vel_a.y), vel_a.x + vel_b.x * cos(vel_b.y - vel_a.y)));
        new_angle = (new_angle + TAU) % TAU;
        return vec2<f32>(new_mag, new_angle);
}

fn get_angle_offset(edge_a_idx: u32, edge_b_idx: u32, primary: bool) -> f32 {
        var left_vertex = edge_vertex_indices[edge_a_idx * 2u];
        var right_vertex = edge_vertex_indices[edge_a_idx * 2u + 1u];
        let b_left_vertex = edge_vertex_indices[edge_b_idx * 2u];
        let b_right_vertex = edge_vertex_indices[edge_b_idx * 2u + 1u];
        // swap the left and right vertices
        if !primary {
                left_vertex = left_vertex ^ right_vertex;
                right_vertex = left_vertex ^ right_vertex;
                left_vertex = left_vertex ^ right_vertex;
        }

        var angle_offset = 0.0;
        // if the edge's primary cell is the same as our primary cell, its reference is pointing mostly downward, so we need to flip it 180 degrees so that the only correction left depends on the number of edges connected to the shared vertex.

        if edge_cell_indices[edge_b_idx * 2u] == edge_cell_indices[edge_a_idx * 2u] && primary {
                angle_offset += PI;
        }
        if edge_cell_indices[edge_b_idx * 2u] == edge_cell_indices[edge_a_idx * 2u + 1u] && !primary {
                angle_offset += PI;
        }

        // The "top" vertex is the one not shared between our edge and the other edge
        var top_vertex = b_right_vertex;
        if top_vertex == left_vertex || top_vertex == right_vertex {
                top_vertex = b_left_vertex;
        }

        // This represents the "left" edge of our triangle
        var angle_sign = 1.0;
        // This is the "right" edge of our triangle
        if right_vertex == b_right_vertex || right_vertex == b_left_vertex {
                angle_sign = -1.0;
        }

        // Determine if we are dealing with a hexagonal or pentagonal cell
        let num_left_edges = vertex_edge_offsets[left_vertex + 1u] - vertex_edge_offsets[left_vertex];
        let num_right_edges = vertex_edge_offsets[right_vertex + 1u] - vertex_edge_offsets[right_vertex];
        let num_top_edges = vertex_edge_offsets[top_vertex + 1u] - vertex_edge_offsets[top_vertex];
        let total_edges = num_left_edges + num_right_edges + num_top_edges;
        // Hexagonal Cell
        if total_edges == 18 {
                angle_offset += angle_sign * (TAU / 6.0);
                return angle_offset;
        }

        // Pentagonal Cell
        if angle_sign > 0.0 {
                // if the vertex is the center of a pentagon, then the angle is 72 degrees
                if num_left_edges == 5 {
                        angle_offset += angle_sign * (TAU / 5.0);
                        return angle_offset;
                }
                // otherwise the angle is 54 degrees
                angle_offset += angle_sign * (PI - TAU / 5.0) / 2.0;
                return angle_offset;
        }

        if num_right_edges == 5 {
                angle_offset += angle_sign * (TAU / 5.0);
                return angle_offset;
        }
        angle_offset += angle_sign * (PI - TAU / 5.0) / 2.0;
        return (angle_offset + TAU) % TAU;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
        // "our" edge
        let edge_idx = global_id.x;
        let num_edges = arrayLength(&velocity_in);

        if (edge_idx >= num_edges) {
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
                let adjusted_velocity = vec2<f32>(primary_edge_velocity.x, (primary_edge_velocity.y + angle_offset) % TAU);

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
                let adjusted_velocity = vec2<f32>(secondary_edge_velocity.x, ((secondary_edge_velocity.y + angle_offset) + TAU) % TAU);

                avg_vel = add_velocity(avg_vel, adjusted_velocity);
        }
        avg_vel.x *= 0.25;

        let neg_velocity = vec2<f32>(velocity_in[edge_idx].x, velocity_in[edge_idx].y + PI);
        var avg_diff = add_velocity(avg_vel, neg_velocity);
        avg_diff.x *= DT * VISCOSITY;
        if abs(avg_diff.x) < EPS {
                velocity_out[edge_idx] = velocity_in[edge_idx];
                return;
        }
        velocity_out[edge_idx] = add_velocity(velocity_in[edge_idx], avg_diff);
}
