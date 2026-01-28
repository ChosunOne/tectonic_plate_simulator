const PI: f32 = 3.1415927;
const TAU: f32 = 6.2831855;
const RHO: f32 = 1.0;
const EPS: f32 = 1.1920929e-7;

struct SimParams {
    dt: f32
}

@group(0) @binding(0) var<storage, read> velocity_in: array<vec2<f32>>;
@group(0) @binding(1) var<storage, read_write> velocity_out: array<vec2<f32>>;

@group(1) @binding(0) var<storage, read> phi_in: array<f32>;
@group(1) @binding(1) var<storage, read_write> phi_out: array<f32>;

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

@group(3) @binding(0) var<uniform> sim_params: SimParams;

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
    let edge_idx = global_id.x;
    let num_edges = arrayLength(&velocity_in);

    if edge_idx >= num_edges {
        return;
    }

    let primary_cell = edge_cell_data[edge_idx * 2u];
    let secondary_cell = edge_cell_data[edge_idx * 2u + 1u];

    let primary_phi = phi_in[primary_cell];
    let secondary_phi = phi_in[secondary_cell];

    if abs(primary_phi - secondary_phi) < EPS {
        return;
    }

    let d = edge_centroid_distance[edge_idx];

    var angle = PI / 2.0;
    if primary_phi < secondary_phi {
        angle = angle + PI;
    }

    let mag = sim_params.dt * abs(primary_phi - secondary_phi) / (RHO * d);

    let vel_adjustment = vec2<f32>(mag, angle);
    velocity_out[edge_idx] = add_velocity(velocity_out[edge_idx], vel_adjustment);
}
