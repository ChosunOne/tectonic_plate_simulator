@group(0) @binding(0) var<storage, read> owned_input: array<u32>;
@group(0) @binding(1) var<storage, read_write> owned_output: array<u32>;

@group(1) @binding(0) var<storage, read> external_input: array<u32>;
@group(1) @binding(1) var<storage, read_write> external_output: array<u32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if index >= arrayLength(&owned_input) {
        return;
    }

    owned_output[index] = owned_input[index] + external_input[index];
}
