use tectonic_plate_simulator::resources::mesh_grid::MeshGrid;

#[test]
fn vertex_edge_adjacency_exists() {
    let grid = MeshGrid::new(20);
    let vertex_edge = grid.vertex_edge_adjacency();
    let num_vertices = grid.points().len();

    assert!(vertex_edge.rows() > 0);

    let mut degree_5_count = 0;
    let mut degree_6_count = 0;

    for v in 0..vertex_edge.rows() {
        let degree = vertex_edge
            .outer_view(v)
            .unwrap()
            .iter()
            .collect::<Vec<_>>()
            .len();
        if degree == 5 {
            degree_5_count += 1;
        } else if degree == 6 {
            degree_6_count += 1;
        } else {
            panic!("Unexpected vertex degree: {degree}");
        }
    }

    assert_eq!(degree_5_count, 12);
    assert_eq!(degree_6_count, num_vertices - 12);
}
