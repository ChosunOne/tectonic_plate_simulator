const PI: f32 = 3.14159265359;
const RHO: f32 = 1.0;
const DT: f32 = 0.01666666667;
@group(0) @binding(0) var<storage, read_write> divergence: array<f32>;

@group(1) @binding(0) var<storage, read> velocity_in: array<vec2<f32>>;
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

fn is_primary(cell_idx: u32, edge_idx: u32) -> bool {
        return cell_idx == edge_cell_indices[edge_idx * 2u];
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
        let cell_idx = global_id.x;
        let num_cells = arrayLength(&divergence);

        if (cell_idx >= num_cells) {
                return;
        }

        var sum: f32 = 0.0;
        for (var i: u32 = 0u; i < 3u; i++) {
                let edge_idx = cell_edge_indices[cell_idx * 3u + i];
                let edge_velocity = velocity_out[edge_idx];
                let mag = edge_velocity.x;
                var angle = edge_velocity.y;
                if !is_primary(cell_idx, edge_idx) {
                        angle += PI;
                }
                sum += mag * sin(angle);
        }
        sum = RHO * sum / DT;
        divergence[cell_idx] = sum;
}
