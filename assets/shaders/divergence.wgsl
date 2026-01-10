const PI: f32 = 3.1415927;
const TAU: f32 = 6.2831855;
const RHO: f32 = 1.0;
const EPS: f32 = 1.1920929e-7;

struct SimParams { dt: f32 };

@group(0) @binding(0) var<storage, read_write> divergence: array<f32>;

@group(1) @binding(0) var<storage, read> velocity_in: array<vec2<f32>>;
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
@group(2) @binding(10) var<storage, read> edge_lengths: array<f32>;
@group(2) @binding(11) var<storage, read> edge_centroid_distance: array<f32>;

@group(3) @binding(0) var<uniform> sim_params: SimParams;

fn is_primary(cell_idx: u32, edge_idx: u32) -> bool {
        return cell_idx == edge_cell_indices[edge_idx * 2u];
}

fn mod_tau(theta: f32) -> f32 {
        if theta >= 0.0 && theta < TAU {
                return theta;
        }
        return (theta + TAU) % TAU;
}

fn cell_area(cell: u32) -> f32 {
        let base_edge = cell_edge_indices[cell * 3u];
        let left_edge = cell_edge_indices[cell * 3u + 1u];
        let right_edge = cell_edge_indices[cell * 3u + 2u];

        let a = edge_lengths[base_edge];
        let b = edge_lengths[left_edge];
        let c = edge_lengths[right_edge];

        let s = (a + b + c) / 2.0;

        return sqrt(s * (s - a) * (s - b) * (s - c));
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
        let cell_idx = global_id.x;
        let num_cells = arrayLength(&divergence);

        if (cell_idx >= num_cells) {
                return;
        }

        if sim_params.dt == 0.0 {
                return;
        }

        divergence[cell_idx] = 0.0;

        var sum: f32 = 0.0;
        for (var i: u32 = 0u; i < 3u; i++) {
                let edge_idx = cell_edge_indices[cell_idx * 3u + i];
                let edge_velocity = velocity_out[edge_idx];
                let mag = edge_velocity.x;
                var angle = edge_velocity.y;
                if !is_primary(cell_idx, edge_idx) {
                        angle = mod_tau(angle + PI);
                }
                if abs(sin(angle)) < EPS || abs(mag) < EPS {
                        continue;
                }
                sum += mag * sin(angle) * edge_lengths[edge_idx];
        }
        sum = RHO * sum / (sim_params.dt * cell_area(cell_idx));
        if abs(sum) < EPS {
                divergence[cell_idx] = 0.0;
                return;
        }
        divergence[cell_idx] = sum;
}
