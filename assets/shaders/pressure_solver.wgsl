@group(0) @binding(0)
var<storage, read> pressure_in: array<f32>;

@group(0) @binding(1)
var<storage, read_write> pressure_out: array<f32>;

@group(0) @binding(2)
var<storage, read> neighbors: array<u32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let num_cells = arrayLength(&pressure_in);
    if idx >= num_cells {
        return;
    }

    let base = idx * 3u;
    let n0 = neighbors[base];
    let n1 = neighbors[base + 1u];
    let n2 = neighbors[base + 2u];

    pressure_out[idx] = (pressure_in[n0] + pressure_in[n1] + pressure_in[n2]) / 3.0;
}
