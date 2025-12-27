@group(0) @binding(0) var<storage, read> vertex_divergence: array<f32>;
@group(0) @binding(1) var<storage, read_write> divergence_bounds: array<f32>;

var<workgroup> local_min: array<f32, 64>;
var<workgroup> local_max: array<f32, 64>;

@compute @workgroup_size(64)
fn main(@builtin(local_invocation_id) local_id: vec3<u32>) {
        let lid = local_id.x;
        let num_vertices = arrayLength(&vertex_divergence);

        var thread_min = 3.40282e+38;
        var thread_max = -3.40282e+38;

        for (var i  =lid; i < num_vertices; i += 64u) {
                let d = vertex_divergence[i];
                thread_min = min(thread_min, d);
                thread_max = max(thread_max, d);
        }

        local_min[lid] = thread_min;
        local_max[lid] = thread_max;

        workgroupBarrier();

        for (var stride   = 32u; stride > 0u; stride >>= 1u) {
                if (lid < stride) {
                        local_min[lid] = min(local_min[lid], local_min[lid + stride]);
                        local_max[lid] = max(local_max[lid], local_max[lid + stride]);
                }
                workgroupBarrier();
        }

        if (lid == 0u) {
                divergence_bounds[0] = local_min[0];
                divergence_bounds[1] = local_max[0];
        }
}
