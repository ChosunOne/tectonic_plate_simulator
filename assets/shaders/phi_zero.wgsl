@group(0) @binding(0) var<storage, read> phi_in: array<f32>;
@group(0) @binding(1) var<storage, read_write> phi_out: array<f32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let cell_idx = global_id.x;
    let num_cells = arrayLength(&phi_in);

    if cell_idx >= num_cells {
        return;
    }

    phi_out[cell_idx] = 0.0;
}
