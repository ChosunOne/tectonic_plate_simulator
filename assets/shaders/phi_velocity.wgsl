const PI: f32 = 3.14159265359;
const RHO: f32 = 1.0;
const DT: f32 = 0.01666666667;

@group(0) @binding(0) var<storage, read> velocity_in: array<vec2<f32>>;
@group(0) @binding(1) var<storage, read_write> velocity_out: array<vec2<f32>>;

@group(1) @binding(0) var<storage, read> phi_in: array<f32>;
@group(1) @binding(0) var<storage, read_write> phi_out: array<f32>;

@group(2) @binding(0) var<storage, read> edge_vertex_indices: array<u32>;
@group(2) @binding(1) var<storage, read> edge_cell_indices: array<u32>;
@group(2) @binding(2) var<storage, read> cell_edge_indices: array<u32>;
@group(2) @binding(3) var<storage, read> cell_vertices: array<u32>;
@group(2) @binding(4) var<storage, read> vertex_edge_offsets: array<u32>;
@group(2) @binding(5) var<storage, read> vertex_edge_indices: array<u32>;
@group(2) @binding(6) var<storage, read> vertex_angle_offsets: array<f32>;
@group(2) @binding(7) var<storage, read> cell_cell_indices: array<u32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
        let edge_idx = global_id.x;
        let num_edges = arrayLength(&velocity_in);

        if (edge_idx >= num_edges) {
                return;
        }

        let primary_phi = phi_in[edge_cell_indices[edge_idx]];
        let secondary_phi = phi_in[edge_cell_indices[edge_idx + 1u]];

        var angle = PI / 2.0;
        if secondary_phi > primary_phi {
                angle += PI;
        }

        let mag = DT * abs(primary_phi - secondary_phi) / RHO;

        let vel = velocity_out[edge_idx];
        let new_mag = sqrt(mag * mag + vel.x * vel.x + 2 * mag * vel.x * cos(angle - vel.y));
        let new_angle = vel.y + atan2(mag * sin(angle - vel.y), vel.x + mag * cos(angle - vel.y));
        velocity_out[edge_idx] = vec2<f32>(new_mag, new_angle);
}
