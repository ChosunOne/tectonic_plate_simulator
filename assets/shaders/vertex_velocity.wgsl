const PI: f32 = 3.1415927;
const TAU: f32 = 6.2831855;
const EPS: f32 = 1.1920929e-7;

@group(0) @binding(0) var<storage, read_write> vertex_velocity: array<vec2<f32>>;

@group(1) @binding(0) var<storage, read> velocity: array<vec2<f32>>;
@group(1) @binding(1) var<storage, read_write> velocity_out: array<vec2<f32>>;

@group(2) @binding(0) var<storage, read> edge_vertex_indices: array<u32>;
@group(2) @binding(1) var<storage, read> edge_cell_indices: array<u32>;
@group(2) @binding(2) var<storage, read> cell_edge_indices: array<u32>;
@group(2) @binding(3) var<storage, read> cell_vertices: array<u32>;
@group(2) @binding(4) var<storage, read> vertex_edge_offsets: array<u32>;
@group(2) @binding(5) var<storage, read> vertex_edge_indices: array<u32>;
@group(2) @binding(6) var<storage, read> vertex_angle_offsets: array<f32>;
@group(2) @binding(7) var<storage, read> cell_cell_indices: array<u32>;
@group(2) @binding(8) var<storage, read> vertex_cell_offsets: array<u32>;
@group(2) @binding(9) var<storage, read> vertex_cell_indices: array<u32>;
@group(2) @binding(10) var<storage, read> edge_lengths: array<f32>;
@group(2) @binding(11) var<storage, read> edge_centroid_distance: array<f32>;

fn mod_tau(theta: f32) -> f32 {
        if theta >= 0.0 && theta < TAU {
                return theta;
        }
        return (theta + TAU) % TAU;
}

// Gets the adjacent edges to this one in the indicated cell.
// Returns (left_edge, right_edge) 
fn get_adjacent_edges(edge: u32, cell: u32) -> vec2<u32> {
        var left_edge: u32;
        var right_edge: u32;
        let is_secondary = cell == edge_cell_indices[edge * 2u + 1u];

        var left_vertex = edge_vertex_indices[edge * 2u];
        var right_vertex = edge_vertex_indices[edge * 2u + 1u];
        if is_secondary {
                left_vertex = left_vertex ^ right_vertex;
                right_vertex = left_vertex ^ right_vertex;
                left_vertex = left_vertex ^ right_vertex;
        }

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

// Find the cell shared by two adjacent edges
fn find_common_cell(edge_idx_a: u32, edge_idx_b: u32) -> u32 {
        // Find the common cell
        let a_primary_cell = edge_cell_indices[edge_idx_a * 2u];
        let a_secondary_cell = edge_cell_indices[edge_idx_a * 2u + 1u];
        let b_primary_cell = edge_cell_indices[edge_idx_b * 2u];
        let b_secondary_cell = edge_cell_indices[edge_idx_b * 2u + 1u];

        var cell: u32;
        if a_primary_cell == b_primary_cell || a_primary_cell == b_secondary_cell  {
                cell = a_primary_cell;
        } else if a_secondary_cell == b_primary_cell || a_secondary_cell == b_secondary_cell {
                cell = a_secondary_cell;
        } else {
                // Edges are not adjacent
                return 1000000u;
        }
        return cell;
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
                (2.0 * a * b), -1.0, 1.0));
        let right_base_angle = acos(clamp((a_squared + c_squared - b_squared) / (2.0 * a * c), -1.0, 1.0));
        let apex_angle = acos(clamp((b_squared + c_squared - a_squared) / (2.0 * b * c), -1.0, 1.0));

        return vec3<f32>(left_base_angle, right_base_angle, apex_angle);
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
        let vertex_idx = global_id.x;
        let num_vertices = arrayLength(&vertex_velocity);

        if (vertex_idx >= num_vertices) {
                return;
        }

        let start = vertex_edge_offsets[vertex_idx];
        let end = vertex_edge_offsets[vertex_idx + 1u];
        let num_edges = end - start;

        if (num_edges == 0u) {
                vertex_velocity[vertex_idx] = vec2<f32>(0.0, 0.0);
                return;
        }

        var sum_x: f32 = 0.0;
        var sum_y: f32 = 0.0;
        var angle_increment = 0.0;
        var prev_edge_idx = 0u;

        for (var i: u32 = 0u; i < num_edges; i++) {
                let edge_idx = vertex_edge_indices[start + i];
                let vel = velocity[edge_idx];
                let mag = vel.x;
                var angle = vel.y;

                let v_lower = edge_vertex_indices[edge_idx * 2u];

                if (vertex_idx != v_lower) {
                        angle = mod_tau(angle + PI);
                }

                if i > 0 {
                        let cell = find_common_cell(prev_edge_idx, edge_idx);
                        let edges = get_adjacent_edges(prev_edge_idx, cell);
                        let base_edge = edge_idx;
                        let right_edge = edges.x;
                        let left_edge = prev_edge_idx; 
                        let angles = compute_angles(base_edge, left_edge, right_edge);
                        angle_increment = mod_tau(angle_increment + angles.x);
                }

                let rotated_angle = mod_tau(angle + angle_increment);

                sum_x += mag * cos(rotated_angle);
                sum_y += mag * sin(rotated_angle);
                prev_edge_idx = edge_idx;
        }

        let avg_x = sum_x / f32(num_edges);
        let avg_y = sum_y / f32(num_edges);

        let avg_mag = sqrt(avg_x * avg_x + avg_y * avg_y);
        if avg_mag < EPS {
                vertex_velocity[vertex_idx] = vec2<f32>(0.0, 0.0);
                return;
        }

        let avg_angle = mod_tau(atan2(avg_y, avg_x));

        vertex_velocity[vertex_idx] = vec2<f32>(avg_mag, avg_angle);
}
