@group(0) @binding(0) var<storage, read_write> vertex_divergence: array<f32>;

@group(1) @binding(0) var<storage, read_write> divergence: array<f32>;


@group(2) @binding(0) var<storage, read> edge_vertex_data: array<u32>;
@group(2) @binding(1) var<storage, read> edge_cell_data: array<u32>;
@group(2) @binding(2) var<storage, read> cell_edge_data: array<u32>;
@group(2) @binding(3) var<storage, read> cell_vertices: array<u32>;
@group(2) @binding(4) var<storage, read> vertex_edge_indices: array<u32>;
@group(2) @binding(5) var<storage, read> vertex_edge_data: array<u32>;
@group(2) @binding(6) var<storage, read> vertex_angle_offsets: array<f32>;
@group(2) @binding(7) var<storage, read> cell_cell_data: array<u32>;
@group(2) @binding(8) var<storage, read> vertex_cell_indices: array<u32>;
@group(2) @binding(9) var<storage, read> vertex_cell_data: array<u32>;
@group(2) @binding(10) var<storage, read> edge_lengths: array<f32>;
@group(2) @binding(11) var<storage, read> edge_centroid_distance: array<f32>;
@group(2) @binding(12) var<storage, read> edge_transport_connection: array<f32>;

fn cell_area(cell: u32) -> f32 {
    let base_edge = cell_edge_data[cell * 3u];
    let left_edge = cell_edge_data[cell * 3u + 1u];
    let right_edge = cell_edge_data[cell * 3u + 2u];

    let a = edge_lengths[base_edge];
    let b = edge_lengths[left_edge];
    let c = edge_lengths[right_edge];

    let s = (a + b + c) / 2.0;

    return sqrt(s * (s - a) * (s - b) * (s - c));
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let vertex_idx = global_id.x;
    let num_vertices = arrayLength(&vertex_divergence);
    if vertex_idx >= num_vertices {
        return;
    }

    let start = vertex_cell_indices[vertex_idx];
    let end = vertex_cell_indices[vertex_idx + 1u];

    var sum = 0.0;
    var area_sum = 0.0;
    for (var i = start; i < end; i++) {
        let cell_idx = vertex_cell_data[i];
        let area = cell_area(cell_idx);
        sum = sum + divergence[cell_idx] * area;
        area_sum = area_sum + area;
    }

    vertex_divergence[vertex_idx] = sum / area_sum;
}
