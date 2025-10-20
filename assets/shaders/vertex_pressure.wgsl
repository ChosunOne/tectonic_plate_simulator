@group(0) @binding(0)
var<storage, read> triangle_pressure: array<f32>;

@group(0) @binding(1)
var<storage, read_write> vertex_pressure: array<f32>;

@group(0) @binding(2)
var<storage, read> vertex_triangles: array<u32>;

const MAX_TRIANGLES_PER_VERTEX: u32 = 8u;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let vertex_idx = global_id.x;
    let num_vertices = arrayLength(&vertex_pressure);
    if vertex_idx >= num_vertices {
        return;
    }

    let base = vertex_idx * MAX_TRIANGLES_PER_VERTEX;
    var sum = 0.0;
    var count = 0u;

    for (var i = 0u; i < MAX_TRIANGLES_PER_VERTEX; i++) {
        let tri_idx = vertex_triangles[base + i];
        if tri_idx != 0xFFFFFFFFu {
            sum += triangle_pressure[tri_idx];
            count++;
        }
    }

    if count > 0u {
        vertex_pressure[vertex_idx] = sum / f32(count);
    } else {
        vertex_pressure[vertex_idx] = 0.0;
    }
}
