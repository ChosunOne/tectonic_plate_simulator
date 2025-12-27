@group(0) @binding(0) var<storage, read_write> vertex_divergence: array<f32>;

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

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
        let vertex_idx = global_id.x;
        let num_vertices = arrayLength(&vertex_divergence);
        if (vertex_idx >= num_vertices) {
                return;
        }

        let start = vertex_cell_offsets[vertex_idx];
        let end = vertex_cell_offsets[vertex_idx + 1u];

        var sum = 0.0;
        for (var i = start; i < end; i++) {
                let cell_idx = vertex_cell_indices[i];
                sum += divergence[cell_idx];
        }

        vertex_divergence[vertex_idx] = sum / f32(end - start);
}
