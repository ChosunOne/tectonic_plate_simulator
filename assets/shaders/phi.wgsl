const EPS: f32 = 1.0e-6;

@group(0) @binding(0) var<storage, read> phi_in: array<f32>;
@group(0) @binding(1) var<storage, read_write> phi_out: array<f32>;

@group(1) @binding(0) var<storage, read_write> divergence: array<f32>;

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

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
        let cell_idx = global_id.x;
        let num_cells = arrayLength(&phi_in);

        if (cell_idx >= num_cells) {
                return;
        }

        var phi: f32 = -divergence[cell_idx];
        for (var i: u32 = 0u; i < 3u; i++) {
                let neighbor_idx = cell_cell_indices[cell_idx * 3u + i];
                phi += phi_in[neighbor_idx];
        }

        phi = phi / 3.0;
        phi_out[cell_idx] = phi;
}
