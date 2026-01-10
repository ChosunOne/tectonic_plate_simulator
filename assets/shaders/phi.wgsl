const EPS: f32 = 1.1920929e-7;

@group(0) @binding(0) var<storage, read> phi_in: array<f32>;
@group(0) @binding(1) var<storage, read_write> phi_out: array<f32>;

@group(1) @binding(0) var<storage, read_write> divergence: array<f32>;

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

// gets the edge dividing the two cells
fn get_dividing_edge(cell_a: u32, cell_b: u32) -> u32 {
        for (var i = 0u; i < 3u; i++) {
                for (var j = 0u; j < 3u; j++) {
                        if cell_edge_indices[cell_a * 3u + i] == cell_edge_indices[cell_b * 3u + j] {
                                return cell_edge_indices[cell_a * 3u + i];
                        }
                }
        }

        // cells are not adjacent
        return 1000000000u;
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
        let num_cells = arrayLength(&phi_in);

        if (cell_idx >= num_cells) {
                return;
        }

        var weight_sum: f32 = 0.0;
        var neighbor_sum: f32 = 0.0;
        for (var i: u32 = 0u; i < 3u; i++) {
                let neighbor_idx = cell_cell_indices[cell_idx * 3u + i];
                let edge_idx = get_dividing_edge(cell_idx, neighbor_idx);
                let w = edge_lengths[edge_idx] / edge_centroid_distance[edge_idx];
                neighbor_sum += w * phi_in[neighbor_idx];
                weight_sum += w;
        }

        let phi = (neighbor_sum - cell_area(cell_idx) * divergence[cell_idx]) / weight_sum;

        if abs(phi) < EPS {
                phi_out[cell_idx] = 0.0;
                return;
        }
        phi_out[cell_idx] = phi;
}
