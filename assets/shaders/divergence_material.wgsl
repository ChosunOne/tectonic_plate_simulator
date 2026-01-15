#import bevy_pbr::{
        mesh_bindings::mesh,
        mesh_functions,
        forward_io::VertexOutput,
        view_transformations::position_world_to_clip,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<storage, read> vertex_divergence: array<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<storage, read> vertex_divergence_bounds: array<f32, 2>;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @builtin(vertex_index) vertex_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

struct VertexDivergenceOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) normalized_divergence: f32,
}

@vertex
fn vertex(vertex: Vertex) -> VertexDivergenceOutput {
    var out: VertexDivergenceOutput;

    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);

    out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));
    out.position = position_world_to_clip(out.world_position.xyz);
    out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal, vertex.instance_index);

    let divergence = vertex_divergence[vertex.vertex_index];
    let min_divergence = vertex_divergence_bounds[0];
    let max_divergence = vertex_divergence_bounds[1];

    let mid_divergence = min_divergence + (max_divergence - min_divergence) / 2.0;
    let deviation = divergence - mid_divergence;
    let max_deviation = max(abs(max_divergence - mid_divergence), abs(min_divergence - mid_divergence));

    if max_deviation > 0.00 {
        let sign = sign(deviation);
        let log_dev = sign * log(1.0 + abs(deviation)) / log(1.0 + max_deviation);
                // out.normalized_divergence = 0.5 + 0.5 * log_dev;
        out.normalized_divergence = (divergence - min_divergence) / (max_divergence - min_divergence);
    } else if max_deviation >= 0.0 {
        out.normalized_divergence = 0.0;
    } else {
        out.normalized_divergence = -1.0;
    }

    return out;
}

fn divergence_to_color(normalized_divergence: f32) -> vec3<f32> {
    if normalized_divergence < 0.0 {
        return vec3<f32>(1.0, 0.0, 1.0);
    } else {
        let t = normalized_divergence;
        return vec3<f32>(t, t, t);
    }
}

@fragment
fn fragment(in: VertexDivergenceOutput) -> @location(0) vec4<f32> {
    let color = divergence_to_color(in.normalized_divergence);
    return vec4<f32>(color, 1.0);
}
