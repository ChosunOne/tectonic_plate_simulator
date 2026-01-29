const PI: f32 = 3.1415927;
const TAU: f32 = 6.2831855;
const EPS: f32 = 1.1920929e-7;
const MAX: u32 = 4294967295u;

@group(0) @binding(0) var<storage, read_write> vertex_velocity: array<vec2<f32>>;

@group(1) @binding(0) var<storage, read> velocity: array<vec2<f32>>;
@group(1) @binding(1) var<storage, read_write> velocity_out: array<vec2<f32>>;

@group(2) @binding(0) var<storage, read> edge_vertex_data: array<u32>;
@group(2) @binding(1) var<storage, read> edge_cell_data: array<u32>;
@group(2) @binding(2) var<storage, read> cell_edge_data: array<u32>;
@group(2) @binding(3) var<storage, read> cell_vertices: array<u32>;
@group(2) @binding(4) var<storage, read> vertex_edge_indices: array<u32>;
@group(2) @binding(5) var<storage, read> vertex_edge_data: array<u32>;
@group(2) @binding(6) var<storage, read> vertex_angle_offsets: array<f32>;
@group(2) @binding(7) var<storage, read> cell_cell_data: array<u32>;
@group(2) @binding(8) var<storage, read> vertex_cell_indices: array<u32>;
@group(2) @binding(9) var<storage, read> vertex_cell_data: array<u32>;
@group(2) @binding(10) var<storage, read> edge_lengths: array<f32>;
@group(2) @binding(11) var<storage, read> edge_centroid_distance: array<f32>;
@group(2) @binding(12) var<storage, read> edge_transport_connection: array<f32>;
@group(2) @binding(13) var<storage, read> edge_parallel_transport_row_indices: array<u32>;
@group(2) @binding(14) var<storage, read> edge_parallel_transport_col_indices: array<u32>;
@group(2) @binding(15) var<storage, read> edge_parallel_transport_data: array<f32>;

fn get_transport_value(row: u32, col: u32) -> f32 {
    var left = edge_parallel_transport_row_indices[row];
    var right = edge_parallel_transport_row_indices[row + 1u] - 1u;
    var first_true_col = MAX;
    while left <= right {
        let mid = left + (right - left) / 2;
        if edge_parallel_transport_col_indices[mid] >= col {
            first_true_col = mid;
            if mid == 0 {
                break;
            }
            right = mid - 1;
        } else {
            left = mid + 1;
        }
    }

    if first_true_col == MAX || edge_parallel_transport_col_indices[first_true_col] != col {
        // return NaN
        let x = -1.0;
        return inverseSqrt(x);
    }

    return edge_parallel_transport_data[first_true_col];
}

fn mod_tau(theta: f32) -> f32 {
    if theta >= 0.0 && theta < TAU {
        return theta;
    }
    return (theta + TAU) % TAU;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let vertex_idx = global_id.x;
    let num_vertices = arrayLength(&vertex_velocity);

    if vertex_idx >= num_vertices {
        return;
    }

    let start = vertex_edge_indices[vertex_idx];
    let end = vertex_edge_indices[vertex_idx + 1u];
    let num_edges = end - start;

    if num_edges == 0u {
        vertex_velocity[vertex_idx] = vec2<f32>(0.0, 0.0);
        return;
    }

    var sum_x: f32 = 0.0;
    var sum_y: f32 = 0.0;
    var angle_increment = 0.0;
    var prev_edge_idx = 0u;

    for (var i: u32 = 0u; i < num_edges; i++) {
        let edge_idx = vertex_edge_data[start + i];
        let vel = velocity[edge_idx];
        let mag = vel.x;
        let angle = vel.y;

        if i > 0 {
            angle_increment = angle_increment + get_transport_value(edge_idx, prev_edge_idx);
        }

        let rotated_angle = mod_tau(angle + angle_increment);

        sum_x = sum_x + mag * cos(rotated_angle);
        sum_y = sum_y + mag * sin(rotated_angle);
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
