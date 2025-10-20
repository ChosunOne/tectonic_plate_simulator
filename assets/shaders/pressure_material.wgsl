#import bevy_pbr::mesh_functions::get_world_from_local
#import bevy_pbr::mesh_view_bindings::view

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<storage, read> vertex_pressure: array<f32>;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @builtin(vertex_index) vertex_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) pressure: f32,
}

@vertex
fn vertex(in: Vertex) -> VertexOutput {
    var out: VertexOutput;

    let world_from_local = get_world_from_local(in.instance_index);
    let world_position = world_from_local * vec4(in.position, 1.0);

    out.position = view.clip_from_world * world_position;
    out.pressure = vertex_pressure[in.vertex_index];

    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let pressure = in.pressure;
    let normalized = clamp(pressure / 8800.0, 0.0, 1.0);

    var color: vec3<f32>;

    if normalized < 0.5 {
        let t = normalized * 2.0;
        color = mix(vec3(0.0, 0.0, 1.0), vec3(0.0, 0.0, 0.0), t);
    } else {
        let t = (normalized - 0.5) * 2.0;
        color = mix(vec3(0.0, 0.0, 0.0), vec3(1.0, 0.0, 0.0), t);
    }

    return vec4(color, 1.0);
}
