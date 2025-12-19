use std::{marker::PhantomData, sync::Arc};

use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    platform::collections::HashMap,
    prelude::*,
    render::extract_resource::ExtractResource,
};
use hexasphere::shapes::IcoSphere;

#[derive(Debug, Clone)]
pub struct CellData {
    pub center: Vec3,
    pub vertices: [u32; 3],
}

/// Cell -> Cell adjacency marker
#[derive(Clone, Debug, Copy, Eq, PartialEq, Hash)]
pub struct Cell;

/// Vertex -> Cell adjacency marker
#[derive(Clone, Debug, Copy, Eq, PartialEq, Hash)]
pub struct VertexCell;

/// Edge -> Cell adjacency marker
#[derive(Clone, Debug, Copy, Eq, PartialEq, Hash)]
pub struct EdgeCell;

/// Edge -> Vertex adjacency marker
#[derive(Clone, Debug, Copy, Eq, PartialEq, Hash)]
pub struct EdgeVertex;

/// Cell -> Edge adjacency marker
#[derive(Clone, Debug, Copy, Eq, PartialEq, Hash)]
pub struct CellEdge;

// NB: Structured this way to allow fast sharing between render and main world
#[derive(Resource, Clone)]
pub struct MantleGrid(Arc<MantleGridInner>);

impl MantleGrid {
    #[must_use]
    pub fn new(subdivisions: usize) -> Self {
        Self(Arc::new(MantleGridInner::new(subdivisions)))
    }

    #[must_use]
    pub fn mesh(&self) -> Mesh {
        self.0.mesh()
    }

    #[must_use]
    pub fn sphere(&self) -> &IcoSphere<()> {
        &self.0.sphere
    }

    #[must_use]
    pub fn cells(&self) -> &[CellData] {
        &self.0.cells
    }

    #[must_use]
    pub fn cell_adjacency(&self) -> &Adjacency<Cell> {
        &self.0.cell_adjacency
    }

    #[must_use]
    pub fn cell_edge_adjacency(&self) -> &Adjacency<CellEdge> {
        &self.0.cell_edge_adjacency
    }

    #[must_use]
    pub fn edge_cell_adjacency(&self) -> &Adjacency<EdgeCell> {
        &self.0.edge_cell_adjacency
    }

    #[must_use]
    pub fn edge_vertex_adjacency(&self) -> &Adjacency<EdgeVertex> {
        &self.0.edge_vertex_adjacency
    }

    #[must_use]
    pub fn vertex_cell_adjacency(&self) -> &Adjacency<VertexCell> {
        &self.0.vertex_cell_adjacency
    }
}

/// CSR Adjacency data
pub struct Adjacency<T> {
    offsets: Vec<u32>,
    indices: Vec<u32>,
    _t: PhantomData<T>,
}

impl<T> Adjacency<T> {
    #[must_use]
    pub fn len(&self) -> usize {
        self.offsets.len().saturating_sub(1)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[must_use]
    pub fn offsets(&self) -> &[u32] {
        &self.offsets
    }

    #[must_use]
    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    pub fn get(&self, idx: usize) -> impl Iterator<Item = usize> + '_ {
        let start = self.offsets[idx] as usize;
        let end = self.offsets[idx + 1] as usize;
        self.indices[start..end].iter().map(|&i| i as usize)
    }

    #[must_use]
    pub fn count(&self, idx: usize) -> usize {
        (self.offsets[idx + 1] - self.offsets[idx]) as usize
    }
}

impl From<&IcoSphere<()>> for Adjacency<Cell> {
    fn from(sphere: &IcoSphere<()>) -> Self {
        let edge_cells = Adjacency::<EdgeCell>::from(sphere);
        let cell_edges = Adjacency::<CellEdge>::from(sphere);
        let num_cells = cell_edges.len();

        let mut offsets = Vec::with_capacity(num_cells + 1);
        let mut indices = Vec::with_capacity(num_cells * 3);

        for cell_idx in 0..num_cells {
            offsets.push(indices.len() as u32);

            for edge_idx in cell_edges.get(cell_idx) {
                let cells: Vec<_> = edge_cells.get(edge_idx).collect();
                let neighbor = if cells[0] == cell_idx {
                    cells[1]
                } else {
                    cells[0]
                };
                indices.push(neighbor as u32);
            }
        }

        offsets.push(indices.len() as u32);

        Self {
            offsets,
            indices,
            _t: PhantomData,
        }
    }
}

impl From<&IcoSphere<()>> for Adjacency<CellEdge> {
    fn from(sphere: &IcoSphere<()>) -> Self {
        let mesh_indices = sphere.get_all_indices();
        let num_cells = mesh_indices.len() / 3;

        let mut edge_map: HashMap<(u32, u32), u32> = HashMap::new();
        let mut next_edge_idx = 0u32;

        for cell_idx in 0..num_cells {
            let base = cell_idx * 3;
            let cell_verts = [
                mesh_indices[base],
                mesh_indices[base + 1],
                mesh_indices[base + 2],
            ];

            for local_edge in 0..3 {
                let v0 = cell_verts[local_edge];
                let v1 = cell_verts[(local_edge + 1) % 3];
                let canonical = (v0.min(v1), v0.max(v1));

                edge_map.entry(canonical).or_insert_with(|| {
                    let idx = next_edge_idx;
                    next_edge_idx += 1;
                    idx
                });
            }
        }
        let mut offsets = Vec::with_capacity(num_cells + 1);
        let mut indices = Vec::with_capacity(num_cells * 3);

        for cell_idx in 0..num_cells {
            offsets.push(indices.len() as u32);

            let base = cell_idx * 3;
            let cell_verts = [
                mesh_indices[base],
                mesh_indices[base + 1],
                mesh_indices[base + 2],
            ];

            for local_edge in 0..3 {
                let v0 = cell_verts[local_edge];
                let v1 = cell_verts[(local_edge + 1) % 3];
                let canonical = (v0.min(v1), v0.max(v1));
                let edge_idx = edge_map[&canonical];
                indices.push(edge_idx);
            }
        }

        offsets.push(indices.len() as u32);

        Self {
            offsets,
            indices,
            _t: PhantomData,
        }
    }
}

impl From<&IcoSphere<()>> for Adjacency<EdgeCell> {
    fn from(sphere: &IcoSphere<()>) -> Self {
        let mesh_indices = sphere.get_all_indices();
        let num_cells = mesh_indices.len() / 3;

        let mut edge_map: HashMap<(u32, u32), (usize, [u32; 2], usize)> = HashMap::new();
        let mut next_edge_idx = 0usize;

        for cell_idx in 0..num_cells {
            let base = cell_idx * 3;
            let cell_verts = [
                mesh_indices[base],
                mesh_indices[base + 1],
                mesh_indices[base + 2],
            ];

            for local_edge in 0..3 {
                let v0 = cell_verts[local_edge];
                let v1 = cell_verts[(local_edge + 1) % 3];
                let canonical = (v0.min(v1), v0.max(v1));
                let is_primary = v0 < v1;

                let entry = edge_map.entry(canonical).or_insert_with(|| {
                    let idx = next_edge_idx;
                    next_edge_idx += 1;
                    (idx, [0, 0], 0)
                });

                let slot = if is_primary { 0 } else { 1 };
                entry.1[slot] = cell_idx as u32;
                entry.2 += 1;
            }
        }

        let num_edges = edge_map.len();

        let mut offsets = Vec::with_capacity(num_edges + 1);
        let mut indices = vec![0u32; num_edges * 2];

        for (edge_idx, cells, _) in edge_map.values() {
            let offset = edge_idx * 2;
            indices[offset] = cells[0];
            indices[offset + 1] = cells[1];
        }

        for i in 0..=num_edges {
            offsets.push((i * 2) as u32);
        }

        Self {
            offsets,
            indices,
            _t: PhantomData,
        }
    }
}

impl From<&IcoSphere<()>> for Adjacency<EdgeVertex> {
    fn from(sphere: &IcoSphere<()>) -> Self {
        let mesh_indices = sphere.get_all_indices();
        let num_cells = mesh_indices.len() / 3;

        let mut edge_set: HashMap<(u32, u32), usize> = HashMap::new();
        let mut next_edge_idx = 0usize;

        for cell_idx in 0..num_cells {
            let base = cell_idx * 3;
            let cell_verts = [
                mesh_indices[base],
                mesh_indices[base + 1],
                mesh_indices[base + 2],
            ];

            for local_edge in 0..3 {
                let v0 = cell_verts[local_edge];
                let v1 = cell_verts[(local_edge + 1) % 3];
                let canonical = (v0.min(v1), v0.max(v1));

                edge_set.entry(canonical).or_insert_with(|| {
                    let idx = next_edge_idx;
                    next_edge_idx += 1;
                    idx
                });
            }
        }

        let num_edges = edge_set.len();

        let mut offsets = Vec::with_capacity(num_edges + 1);
        let mut indices = vec![0u32; num_edges * 2];

        for ((v_lower, v_higher), edge_idx) in &edge_set {
            let offset = edge_idx * 2;
            indices[offset] = *v_lower;
            indices[offset + 1] = *v_higher;
        }

        for i in 0..=num_edges {
            offsets.push((i * 2) as u32);
        }

        Self {
            offsets,
            indices,
            _t: PhantomData,
        }
    }
}

impl From<&IcoSphere<()>> for Adjacency<VertexCell> {
    fn from(sphere: &IcoSphere<()>) -> Self {
        let points = sphere.raw_points();
        let mesh_indices = sphere.get_all_indices();
        let num_vertices = points.len();
        let num_cells = mesh_indices.len() / 3;

        let mut counts = vec![0u32; num_vertices];
        for cell_idx in 0..num_cells {
            let base = cell_idx * 3;
            counts[mesh_indices[base] as usize] += 1;
            counts[mesh_indices[base + 1] as usize] += 1;
            counts[mesh_indices[base + 2] as usize] += 1;
        }

        let mut offsets = Vec::with_capacity(num_vertices + 1);
        let mut running = 0u32;
        for &count in &counts {
            offsets.push(running);
            running += count;
        }
        offsets.push(running);

        let mut write_pos = offsets[..num_vertices].to_vec();
        let mut indices = vec![0u32; running as usize];

        for cell_idx in 0..num_cells {
            let base = cell_idx * 3;
            for i in 0..3 {
                let v = mesh_indices[base + i] as usize;
                indices[write_pos[v] as usize] = cell_idx as u32;
                write_pos[v] += 1;
            }
        }

        Self {
            offsets,
            indices,
            _t: PhantomData,
        }
    }
}

struct MantleGridInner {
    pub cell_adjacency: Adjacency<Cell>,
    pub cell_edge_adjacency: Adjacency<CellEdge>,
    pub cells: Vec<CellData>,
    pub edge_cell_adjacency: Adjacency<EdgeCell>,
    pub edge_vertex_adjacency: Adjacency<EdgeVertex>,
    pub sphere: IcoSphere<()>,
    pub vertex_cell_adjacency: Adjacency<VertexCell>,
}

impl ExtractResource for MantleGrid {
    type Source = MantleGrid;

    fn extract_resource(source: &Self::Source) -> Self {
        source.clone()
    }
}

impl MantleGridInner {
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn new(subdivisions: usize) -> Self {
        let sphere = IcoSphere::new(subdivisions, |_| {});
        let points = sphere.raw_points();
        let indices = sphere.get_all_indices();
        let num_triangles = indices.len() / 3;

        let cell_adjacency = Adjacency::<Cell>::from(&sphere);
        let cell_edge_adjacency = Adjacency::<CellEdge>::from(&sphere);
        let edge_cell_adjacency = Adjacency::<EdgeCell>::from(&sphere);
        let edge_vertex_adjacency = Adjacency::<EdgeVertex>::from(&sphere);
        let vertex_cell_adjacency = Adjacency::<VertexCell>::from(&sphere);

        let mut cells = Vec::new();
        for tri_idx in 0..num_triangles {
            let base = tri_idx * 3;
            let v0 = indices[base];
            let v1 = indices[base + 1];
            let v2 = indices[base + 2];

            let p0: Vec3 = points[v0 as usize].into();
            let p1: Vec3 = points[v1 as usize].into();
            let p2: Vec3 = points[v2 as usize].into();

            let center = ((p0 + p1 + p2) / 3.0).normalize();

            cells.push(CellData {
                center,
                vertices: [v0, v1, v2],
            });
        }

        Self {
            cell_adjacency,
            cell_edge_adjacency,
            cells,
            edge_cell_adjacency,
            edge_vertex_adjacency,
            sphere,
            vertex_cell_adjacency,
        }
    }

    #[must_use]
    pub fn mesh(&self) -> Mesh {
        let points = self.sphere.raw_points();
        let indices = self.sphere.get_all_indices();

        let positions = points.iter().map(|&p| p.into()).collect::<Vec<[f32; 3]>>();
        let normals = points
            .iter()
            .map(|&p| p.normalize().into())
            .collect::<Vec<[f32; 3]>>();

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all());

        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_indices(Indices::U32(indices));
        mesh
    }
}
