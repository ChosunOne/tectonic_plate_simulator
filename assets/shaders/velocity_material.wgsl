#import bevy_pbr::{
    mesh_bindings::mesh,
    mesh_functions,
    forward_io::VertexOutput,
    view_transformations::position_world_to_clip,
}

const PI: f32 = 3.14159265359;
const TAU: f32 = 6.28318530718;

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<storage, read> vertex_velocity: array<vec2<f32>>;

@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<storage, read> vertex_velocity_bounds: array<f32, 2>;

@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<storage, read> vertex_angle_offsets: array<f32>;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @builtin(vertex_index) vertex_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

struct VertexVelocityOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) normalized_magnitude: f32,
    @location(3) velocity_x: f32,
    @location(4) velocity_y: f32,
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> vec3<f32> {
        let c = v * s;
        let hp = h * 6.0;
        let x = c * (1.0 - abs(hp % 2.0 - 1.0));
        let m = v - c;

        var rgb: vec3<f32>;
        if hp < 1.0 {
                rgb = vec3<f32>(c, x, 0.0);
        } else if hp < 2.0 {
                rgb = vec3<f32>(x, c, 0.0);
        } else if hp < 3.0 {
                rgb = vec3<f32>(0.0, c, x);
        } else if hp < 4.0 {
                rgb = vec3<f32>(0.0, x, c);
        } else if hp < 5.0 {
                rgb = vec3<f32>(x, 0.0, c);
        } else {
                rgb = vec3<f32>(c, 0.0, x);
        }

        return rgb + vec3<f32>(m, m, m);
}

@vertex
fn vertex(vertex: Vertex) -> VertexVelocityOutput {
        var out: VertexVelocityOutput;

        let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);

        out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));
        out.position = position_world_to_clip(out.world_position.xyz);
        out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal, vertex.instance_index);

        let vel = vertex_velocity[vertex.vertex_index];
        let magnitude = vel.x;
        let angle = vel.y;

        let min_mag = vertex_velocity_bounds[0];
        let max_mag = vertex_velocity_bounds[1];
        let range = max_mag - min_mag;

        if range > 0.0 {
                out.normalized_magnitude = (magnitude - min_mag) / range;
        } else {
                out.normalized_magnitude = 0.5;
        }

        let angle_offset = vertex_angle_offsets[vertex.vertex_index];
        let global_angle = angle + angle_offset;
        out.velocity_x = cos(global_angle);
        out.velocity_y = sin(global_angle);

        return out;
}

@fragment
fn fragment(in: VertexVelocityOutput) -> @location(0) vec4<f32> {
        let angle = atan2(in.velocity_y, in.velocity_x);
        var hue = (angle + PI) / TAU;
        hue = hue - floor(hue);
        let saturation = 1.0;
        let value = in.normalized_magnitude;
        let color = hsv_to_rgb(hue, saturation, value);
        return vec4<f32>(color, 1.0);
}
