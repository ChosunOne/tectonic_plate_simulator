const PI: f32 = 3.1415927;
const TAU: f32 = 6.2831855;
const VISCOSITY: f32 = 0.01;
const EPS: f32 = 1.1920929e-7;
const MAX: u32 = 4294967295u;

struct SimParams {
    dt: f32
}

@group(0) @binding(0) var<storage, read> velocity_in: array<vec2<f32>>;
@group(0) @binding(1) var<storage, read_write> velocity_out: array<vec2<f32>>;

@group(1) @binding(0) var<storage, read> edge_vertex_data: array<u32>;
@group(1) @binding(1) var<storage, read> edge_cell_data: array<u32>;
@group(1) @binding(2) var<storage, read> cell_edge_data: array<u32>;
@group(1) @binding(3) var<storage, read> cell_vertices: array<u32>;
@group(1) @binding(4) var<storage, read> vertex_edge_indices: array<u32>;
@group(1) @binding(5) var<storage, read> vertex_edge_data: array<u32>;
@group(1) @binding(6) var<storage, read> vertex_angle_offsets: array<f32>;
@group(1) @binding(7) var<storage, read> cell_cell_data: array<u32>;
@group(1) @binding(8) var<storage, read> vertex_cell_indices: array<u32>;
@group(1) @binding(9) var<storage, read> vertex_cell_data: array<u32>;
@group(1) @binding(10) var<storage, read> edge_lengths: array<f32>;
@group(1) @binding(11) var<storage, read> edge_centroid_distance: array<f32>;
@group(1) @binding(12) var<storage, read> edge_transport_connection: array<f32>;
@group(1) @binding(13) var<storage, read> edge_transport_row_indices: array<u32>;
@group(1) @binding(14) var<storage, read> edge_transport_col_indices: array<u32>;
@group(1) @binding(15) var<storage, read> edge_transport_data: array<f32>;

@group(2) @binding(0) var<uniform> sim_params: SimParams; 

fn get_transport_value(row: u32, col: u32) -> f32 {
    var left = edge_transport_row_indices[row];
    var right = edge_transport_row_indices[row + 1u] - 1u;
    var first_true_col = MAX;
    while left <= right {
        let mid = left + (right - left) / 2;
        if edge_transport_col_indices[mid] >= col {
            first_true_col = mid;
            if mid == 0 {
                break;
            }
            right = mid - 1;
        } else {
            left = mid + 1;
        }
    }

    if first_true_col == MAX || edge_transport_col_indices[first_true_col] != col {
        // return NaN
        let x = -1.0;
        return inverseSqrt(x);
    }

    return edge_transport_data[first_true_col];
}

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


@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
        // "our" edge
    let edge_idx = global_id.x;
    let num_edges = arrayLength(&velocity_in);

    if edge_idx >= num_edges {
        return;
    }

    if VISCOSITY == 0.0 {
        velocity_out[edge_idx] = velocity_in[edge_idx];
        return;
    }

    let primary_cell = edge_cell_data[edge_idx * 2u];
    let secondary_cell = edge_cell_data[edge_idx * 2u + 1u];

    var avg_vel = vec2<f32>(0.0, 0.0);

    for (var i: u32 = 0u; i < 3; i++) {
        let primary_edge_idx = cell_edge_data[primary_cell * 3u + i];
        // only average the *other* velocities surrounding the one we want to update.
        if primary_edge_idx == edge_idx {
            continue;
        }

        let angle_offset = get_transport_value(primary_edge_idx, edge_idx);

        let primary_edge_velocity = velocity_in[primary_edge_idx];
        let adjusted_velocity = vec2<f32>(primary_edge_velocity.x, mod_tau(primary_edge_velocity.y + angle_offset));

        avg_vel = add_velocity(avg_vel, adjusted_velocity);
    }

    for (var i: u32 = 0; i < 3; i++) {
        let secondary_edge_idx = cell_edge_data[secondary_cell * 3u + i];
        if secondary_edge_idx == edge_idx {
            continue;
        }

        let angle_offset = get_transport_value(secondary_edge_idx, edge_idx);

        let secondary_edge_velocity = velocity_in[secondary_edge_idx];
        let adjusted_velocity = vec2<f32>(secondary_edge_velocity.x, mod_tau(secondary_edge_velocity.y + angle_offset - edge_transport_connection[edge_idx]));

        avg_vel = add_velocity(avg_vel, adjusted_velocity);
    }
    avg_vel.x = avg_vel.x * 0.25;

    let neg_velocity = vec2<f32>(velocity_in[edge_idx].x, velocity_in[edge_idx].y + PI);
    var avg_diff = add_velocity(avg_vel, neg_velocity);
    avg_diff.x = avg_diff.x * sim_params.dt * VISCOSITY;
    if abs(avg_diff.x) < EPS {
        velocity_out[edge_idx] = velocity_in[edge_idx];
        return;
    }
    velocity_out[edge_idx] = add_velocity(velocity_in[edge_idx], avg_diff);
}
