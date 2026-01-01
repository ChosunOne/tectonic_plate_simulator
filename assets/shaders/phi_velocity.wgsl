const PI: f32 = 3.14159265359;
const TAU: f32 = 6.283185230718;
const RHO: f32 = 1.0;
const DT: f32 = 0.01666666667;
const EPS: f32 = 1e-10;

@group(0) @binding(0) var<storage, read> velocity_in: array<vec2<f32>>;
@group(0) @binding(1) var<storage, read_write> velocity_out: array<vec2<f32>>;

@group(1) @binding(0) var<storage, read> phi_in: array<f32>;
@group(1) @binding(1) var<storage, read_write> phi_out: array<f32>;

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
        return vec2<f32>(new_mag, (new_angle + TAU) % TAU);
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
        let edge_idx = global_id.x;
        let num_edges = arrayLength(&velocity_in);

        if (edge_idx >= num_edges) {
                return;
        }

        let primary_phi = phi_in[edge_cell_indices[edge_idx * 2u]];
        let secondary_phi = phi_in[edge_cell_indices[edge_idx * 2u + 1u]];

        if abs(primary_phi - secondary_phi) < EPS {
                velocity_out[edge_idx] = velocity_in[edge_idx];
                return;
        }

        var angle = PI / 2.0;
        if primary_phi < secondary_phi {
                angle += PI;
        }

        let mag = DT * abs(primary_phi - secondary_phi) / RHO;

        let vel = velocity_out[edge_idx];
        let vel_adjustment = vec2<f32>(mag, angle);
        velocity_out[edge_idx] = add_velocity(velocity_out[edge_idx], vel_adjustment);
}
