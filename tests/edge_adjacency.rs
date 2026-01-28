use tectonic_plate_simulator::resources::mesh_grid::MeshGrid;

#[test]
fn test_edge_count_euler_formula() {
    let grid = MeshGrid::new(20);
    let num_vertices = grid.sphere().raw_points().len();
    let num_faces = grid.cells().len();
    let num_edges = grid.edge_cell_adjacency().rows();

    assert_eq!(num_edges, num_vertices + num_faces - 2);
}

#[test]
fn test_every_edge_has_two_cells() {
    let grid = MeshGrid::new(20);
    let edge_cells = grid.edge_cell_adjacency();

    for edge_idx in 0..edge_cells.rows() {
        assert_eq!(
            edge_cells
                .outer_view(edge_idx)
                .unwrap()
                .iter()
                .collect::<Vec<_>>()
                .len(),
            2,
            "Edge {edge_idx} should have exactly 2 cells"
        );
    }
}

#[test]
fn test_every_edge_has_two_vertices() {
    let grid = MeshGrid::new(20);
    let edge_vertices = grid.edge_vertex_adjacency();

    for edge_idx in 0..edge_vertices.rows() {
        assert_eq!(
            edge_vertices
                .outer_view(edge_idx)
                .unwrap()
                .iter()
                .collect::<Vec<_>>()
                .len(),
            2,
            "Edge {edge_idx} should have exactly 2 vertices"
        );
    }
}

#[test]
fn test_edge_vertices_canonical_order() {
    let grid = MeshGrid::new(20);
    let edge_vertices = grid.edge_vertex_adjacency();

    for edge_idx in 0..edge_vertices.rows() {
        let verts: Vec<_> = edge_vertices
            .outer_view(edge_idx)
            .unwrap()
            .iter()
            .map(|(_, &x)| x)
            .collect::<Vec<_>>();

        assert!(
            verts[0] < verts[1],
            "Edge {edge_idx} vertices not in canonical order: {} >= {}",
            verts[0],
            verts[1]
        );
    }
}

#[test]
fn test_every_cell_has_three_edges() {
    let grid = MeshGrid::new(20);
    let cell_edges = grid.cell_edge_adjacency();

    for cell_idx in 0..grid.cells().len() {
        assert_eq!(
            cell_edges
                .outer_view(cell_idx)
                .unwrap()
                .iter()
                .collect::<Vec<_>>()
                .len(),
            3,
            "Cell {cell_idx} should have exactly 3 edges"
        );
    }
}

#[test]
fn test_cell_edge_bidirectional_consistency() {
    let grid = MeshGrid::new(20);
    let cell_edges = grid.cell_edge_adjacency();
    let edge_cells = grid.edge_cell_adjacency();

    for cell_idx in 0..grid.cells().len() {
        for edge_idx in cell_edges
            .outer_view(cell_idx)
            .unwrap()
            .iter()
            .map(|(_, &x)| x as usize)
        {
            let cells: Vec<_> = edge_cells
                .outer_view(edge_idx)
                .unwrap()
                .iter()
                .map(|(_, &x)| x as usize)
                .collect::<Vec<_>>();
            assert!(
                cells.contains(&cell_idx),
                "Cell {cell_idx} claims edge {edge_idx}, but edge has cells {cells:?}"
            );
        }
    }

    for edge_idx in 0..edge_cells.rows() {
        for cell_idx in edge_cells
            .outer_view(edge_idx)
            .unwrap()
            .iter()
            .map(|(_, &x)| x as usize)
        {
            let edges: Vec<_> = cell_edges
                .outer_view(cell_idx)
                .unwrap()
                .iter()
                .map(|(_, &x)| x as usize)
                .collect::<Vec<_>>();
            assert!(
                edges.contains(&edge_idx),
                "Edge {edge_idx} claims cell {cell_idx}, but cell has edges {edges:?}"
            );
        }
    }
}

#[test]
fn test_is_secondary_derivation() {
    let grid = MeshGrid::new(20);
    let cell_edges = grid.cell_edge_adjacency();
    let edge_cells = grid.edge_cell_adjacency();

    for cell_idx in 0..grid.cells().len() {
        let cell_verts = grid.cells()[cell_idx].vertices;

        for (local_edge, edge_idx) in cell_edges
            .outer_view(cell_idx)
            .unwrap()
            .iter()
            .map(|(i, &x)| (i, x as usize))
        {
            let v0 = cell_verts[local_edge];
            let v1 = cell_verts[(local_edge + 1) % 3];

            let is_secondary_derived = v0 > v1;

            let cells: Vec<_> = edge_cells
                .outer_view(edge_idx)
                .unwrap()
                .iter()
                .map(|(_, &x)| x as usize)
                .collect::<Vec<_>>();
            let is_secondary_actual = cells[1] == cell_idx;

            assert_eq!(
                is_secondary_derived, is_secondary_actual,
                "Cell {cell_idx} local_edge {local_edge} (edge {edge_idx}): derived is_scondary={is_secondary_derived}, actual={is_secondary_actual}, cell_verts={cell_verts:?}, edge_cells={cells:?}"
            );
        }
    }
}

#[test]
fn test_edge_cell_primary_ordering() {
    let grid = MeshGrid::new(20);
    let edge_cells = grid.edge_cell_adjacency();
    let edge_vertices = grid.edge_vertex_adjacency();

    for edge_idx in 0..edge_cells.rows() {
        let cells: Vec<_> = edge_cells
            .outer_view(edge_idx)
            .unwrap()
            .iter()
            .map(|(_, &x)| x as usize)
            .collect::<Vec<_>>();
        let verts: Vec<_> = edge_vertices
            .outer_view(edge_idx)
            .unwrap()
            .iter()
            .map(|(_, &x)| x as usize)
            .collect::<Vec<_>>();
        let primary_cell = cells[0];

        let primary_cell_verts = grid.cells()[primary_cell].vertices;
        let mut found = false;

        for local_edge in 0..3 {
            let v0 = primary_cell_verts[local_edge] as usize;
            let v1 = primary_cell_verts[(local_edge + 1) % 3] as usize;

            if v0 == verts[0] && v1 == verts[1] || v0 == verts[1] && v1 == verts[0] {
                assert!(
                    v0 < v1,
                    "Edge {edge_idx}: primary cell {primary_cell} has v0={v0} >= v1={v1}"
                );
                found = true;
                break;
            }
        }

        assert!(
            found,
            "Edge {edge_idx}: couldn't find matching local edge in primary cell {primary_cell}"
        );
    }
}
