@group(0) @binding(0) var<storage, read> vertex_cell_offsets: array<u32>;
@group(0) @binding(1) var<storage, read> vertex_cell_indices: array<u32>;
@group(0) @binding(2) var<storage, read_write> vertex_pressures: array<f32>;

@group(1) @binding(0) var<storage, read> pressure_in: array<f32>;
@group(1) @binding(1) var<storage, read_write> pressure_out: array<f32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
        let vertex_idx = global_id.x;
        let num_vertices = arrayLength(&vertex_pressures);
        if (vertex_idx >= num_vertices) {
                return;
        }

        let start = vertex_cell_offsets[vertex_idx];
        let end = vertex_cell_offsets[vertex_idx + 1u];

        var sum = 0.0;
        for (var i = start; i < end; i++) {
                let cell_idx = vertex_cell_indices[i];
                sum += pressure_out[cell_idx];
        }

        vertex_pressures[vertex_idx] = sum / f32(end - start);
}
