const EPS: f32 = 1e-3;
@group(0) @binding(0) var<storage, read> pressure_in: array<f32>;
@group(0) @binding(1) var<storage, read_write> pressure_out: array<f32>;

@group(1) @binding(0) var<storage, read> phi_in: array<f32>;
@group(1) @binding(1) var<storage, read_write> phi_out: array<f32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
        let cell_idx = global_id.x;
        let num_cells = arrayLength(&pressure_in);

        if (cell_idx >= num_cells) {
                return;
        }

        if (abs(phi_in[cell_idx]) < EPS) {
                return;
        }

        pressure_out[cell_idx] = 0.99 * pressure_out[cell_idx] + phi_in[cell_idx];
}
