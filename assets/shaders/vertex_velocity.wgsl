const PI: f32 = 3.14159265359;
const TAU: f32 = 6.283185230718;
const EPS: f32 = 1e-10;

@group(0) @binding(0) var<storage, read_write> vertex_velocity: array<vec2<f32>>;

@group(1) @binding(0) var<storage, read> velocity: array<vec2<f32>>;
@group(1) @binding(1) var<storage, read_write> velocity_out: array<vec2<f32>>;

@group(2) @binding(0) var<storage, read> edge_vertex_indices: array<u32>;
@group(2) @binding(1) var<storage, read> edge_cell_indices: array<u32>;
@group(2) @binding(2) var<storage, read> cell_edge_indices: array<u32>;
@group(2) @binding(3) var<storage, read> cell_vertices: array<u32>;
@group(2) @binding(4) var<storage, read> vertex_edge_offsets: array<u32>;
@group(2) @binding(5) var<storage, read> vertex_edge_indices: array<u32>;
@group(2) @binding(6) var<storage, read> vertex_angle_offsets: array<f32>;
@group(2) @binding(7) var<storage, read> cell_cell_indices: array<u32>;
@group(2) @binding(8) var<storage, read> vertex_cell_offsets: array<u32>;
@group(2) @binding(9) var<storage, read> vertex_cell_indices: array<u32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
        let vertex_idx = global_id.x;
        let num_vertices = arrayLength(&vertex_velocity);

        if (vertex_idx >= num_vertices) {
                return;
        }

        let start = vertex_edge_offsets[vertex_idx];
        let end = vertex_edge_offsets[vertex_idx + 1u];
        let num_edges = end - start;

        if (num_edges == 0u) {
                vertex_velocity[vertex_idx] = vec2<f32>(0.0, 0.0);
                return;
        }

        let angle_increment = TAU / f32(num_edges);

        var sum_x: f32 = 0.0;
        var sum_y: f32 = 0.0;

        for (var i: u32 = 0u; i < num_edges; i++) {
                let edge_idx = vertex_edge_indices[start + i];
                let vel = velocity[edge_idx];
                let mag = vel.x;
                var angle = vel.y;

                let v_lower = edge_vertex_indices[edge_idx * 2u];

                if (vertex_idx != v_lower) {
                        angle = angle + PI;
                }

                let rotated_angle = angle + f32(i) * angle_increment;

                sum_x += mag * cos(rotated_angle);
                sum_y += mag * sin(rotated_angle);
        }

        let avg_x = sum_x / f32(num_edges);
        let avg_y = sum_y / f32(num_edges);

        let avg_mag = sqrt(avg_x * avg_x + avg_y * avg_y);
        if avg_mag < EPS {
                vertex_velocity[vertex_idx] = vec2<f32>(0.0, 0.0);
                return;
        }

        let avg_angle = atan2(avg_y, avg_x) % (2.0 * PI);

        vertex_velocity[vertex_idx] = vec2<f32>(avg_mag, avg_angle);
}
