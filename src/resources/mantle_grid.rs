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

#[derive(Clone, Debug, Copy, Eq, PartialEq, Hash)]
pub struct Cell;

#[derive(Clone, Debug, Copy, Eq, PartialEq, Hash)]
pub struct VertexCell;

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
        let mesh_indices = sphere.get_all_indices();
        let num_triangles = mesh_indices.len() / 3;

        let mut edge_to_triangles = HashMap::new();
        let mut edge_counts = HashMap::new();

        for tri_idx in 0..num_triangles {
            let base = tri_idx * 3;
            let v0 = mesh_indices[base];
            let v1 = mesh_indices[base + 1];
            let v2 = mesh_indices[base + 2];

            let edges = [
                (v0.min(v1), v0.max(v1)),
                (v1.min(v2), v1.max(v2)),
                (v2.min(v0), v2.max(v0)),
            ];

            for edge in edges {
                let count = edge_counts.entry(edge).or_insert(0);
                let tris = edge_to_triangles.entry(edge).or_insert([0, 0]);
                tris[*count] = tri_idx;
                *count += 1;
            }
        }

        let mut offsets = Vec::with_capacity(num_triangles + 1);
        let mut indices = Vec::with_capacity(num_triangles * 3);

        for tri_idx in 0..num_triangles {
            offsets.push(indices.len() as u32);

            let base = tri_idx * 3;
            let v0 = mesh_indices[base];
            let v1 = mesh_indices[base + 1];
            let v2 = mesh_indices[base + 2];

            let edges = [
                (v0.min(v1), v0.max(v1)),
                (v1.min(v2), v1.max(v2)),
                (v2.min(v0), v2.max(v0)),
            ];

            for edge in edges {
                let tris = &edge_to_triangles[&edge];
                let neighbor = if tris[0] == tri_idx { tris[1] } else { tris[0] };
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

impl From<&IcoSphere<()>> for Adjacency<VertexCell> {
    fn from(sphere: &IcoSphere<()>) -> Self {
        let points = sphere.raw_points();
        let mesh_indices = sphere.get_all_indices();
        let num_vertices = points.len();
        let num_triangles = mesh_indices.len() / 3;

        let mut counts = vec![0u32; num_vertices];
        for tri_idx in 0..num_triangles {
            let base = tri_idx * 3;
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

        for tri_idx in 0..num_triangles {
            let base = tri_idx * 3;
            for i in 0..3 {
                let v = mesh_indices[base + i] as usize;
                indices[write_pos[v] as usize] = tri_idx as u32;
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
    pub cells: Vec<CellData>,
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
            sphere,
            cells,
            cell_adjacency,
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
