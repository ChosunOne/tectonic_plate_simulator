#import bevy_pbr::{
        mesh_bindings::mesh,
        mesh_functions,
        forward_io::VertexOutput,
        view_transformations::position_world_to_clip,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<storage, read> vertex_pressure: array<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<storage, read> vertex_pressure_bounds: array<f32, 2>;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @builtin(vertex_index) vertex_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

struct VertexPressureOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) normalized_pressure: f32,
}

@vertex
fn vertex(vertex: Vertex) -> VertexPressureOutput {
    var out: VertexPressureOutput;

    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);

    out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));
    out.position = position_world_to_clip(out.world_position.xyz);
    out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal, vertex.instance_index);

    let pressure = vertex_pressure[vertex.vertex_index];
    let min_pressure = vertex_pressure_bounds[0];
    let max_pressure = vertex_pressure_bounds[1];

    let mid_pressure = min_pressure + (max_pressure - min_pressure) / 2.0;
    let deviation = pressure - mid_pressure;
    let max_deviation = max(abs(max_pressure - mid_pressure), abs(min_pressure - mid_pressure));

    if max_deviation > 0.0 {
        out.normalized_pressure = (pressure - min_pressure) / (max_pressure - min_pressure);
    } else if max_deviation >= 0.0 {
        out.normalized_pressure = 0.5;
    } else {
        out.normalized_pressure = -1.0;
    }

    return out;
}

fn pressure_to_color(normalized_pressure: f32) -> vec3<f32> {
    if normalized_pressure < 0.5 {
        let t = normalized_pressure * 2.0;
        return vec3<f32>(0.00, 0.00, 1.0 - t);
    } else if normalized_pressure >= 0.0 {
        var t = (normalized_pressure - 0.5) * 2.0;
        return vec3<f32>(t, 0.00, 0.00);
    } else {
        return vec3<f32>(1.0, 0.0, 1.0);
    }
}

@fragment
fn fragment(in: VertexPressureOutput) -> @location(0) vec4<f32> {
    let color = pressure_to_color(in.normalized_pressure);
    return vec4<f32>(color, 1.0);
}
