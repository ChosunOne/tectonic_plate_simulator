use std::collections::HashMap;

use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
    render::extract_resource::ExtractResource,
};
use hexasphere::shapes::IcoSphere;

#[derive(Debug, Clone)]
pub struct CellData {
    pub center: Vec3,
    pub flux: Vec<f32>,
    pub pressure: f32,
    pub vertices: [u32; 3],
    pub edges: [i32; 3],
    pub barycentric_gradients: [Vec3; 3],
    pub edge_normals: [Vec3; 3],
    pub area: f32,
}

#[derive(Clone)]
pub struct Edge {
    pub vertices: (u32, u32),
    pub triangles: (u32, u32),
    pub midpoint: Vec3,
    pub normal: Vec3,
}

#[derive(Resource, Clone)]
pub struct MantleGrid {
    pub sphere: IcoSphere<()>,
    pub cells: Vec<CellData>,
    pub neighbors: Vec<Vec<usize>>,
    pub vertex_triangles: Vec<Vec<usize>>,
    pub edges: Vec<Edge>,
}

impl ExtractResource for MantleGrid {
    type Source = MantleGrid;

    fn extract_resource(source: &Self::Source) -> Self {
        source.clone()
    }
}

impl MantleGrid {
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn new(subdivisions: usize) -> Self {
        let sphere = IcoSphere::new(subdivisions, |_| {});
        let points = sphere.raw_points();
        let indices = sphere.get_all_indices();
        let num_triangles = indices.len() / 3;

        // Build adjacency: map edges to triangles
        let mut edge_to_triangles: HashMap<(u32, u32), Vec<usize>> = HashMap::new();

        for tri_idx in 0..num_triangles {
            let base = tri_idx * 3;
            let v0 = indices[base];
            let v1 = indices[base + 1];
            let v2 = indices[base + 2];

            // Add three edges (sorted for consistent lookup)
            let edges = [
                (v0.min(v1), v0.max(v1)),
                (v1.min(v2), v1.max(v2)),
                (v2.min(v0), v2.max(v0)),
            ];

            for edge in edges {
                edge_to_triangles.entry(edge).or_default().push(tri_idx);
            }
        }

        // Build neighbor list for each triangle
        let mut neighbors = vec![Vec::new(); num_triangles];
        for tri_idx in 0..num_triangles {
            let base = tri_idx * 3;
            let v0 = indices[base];
            let v1 = indices[base + 1];
            let v2 = indices[base + 2];

            let edges = [
                (v0.min(v1), v0.max(v1)),
                (v1.min(v2), v1.max(v2)),
                (v2.min(v0), v2.max(v0)),
            ];

            for edge in edges {
                if let Some(tris) = edge_to_triangles.get(&edge) {
                    for &neighbor_idx in tris {
                        if neighbor_idx != tri_idx {
                            neighbors[tri_idx].push(neighbor_idx);
                        }
                    }
                }
            }
        }

        let num_vertices = points.len();
        let mut vertex_triangles = vec![Vec::new(); num_vertices];
        for tri_idx in 0..num_triangles {
            let base = tri_idx * 3;
            let v0 = indices[base] as usize;
            let v1 = indices[base + 1] as usize;
            let v2 = indices[base + 2] as usize;

            vertex_triangles[v0].push(tri_idx);
            vertex_triangles[v1].push(tri_idx);
            vertex_triangles[v2].push(tri_idx);
        }

        let mut edges = Vec::new();
        let mut edge_map = HashMap::new();

        for (edge_verts, tri_list) in &edge_to_triangles {
            debug_assert_eq!(
                tri_list.len(),
                2,
                "Edge must have exactly 2 triangles on sphere"
            );

            let v0_pos = points[edge_verts.0 as usize];
            let v1_pos = points[edge_verts.1 as usize];

            let midpoint: Vec3 = ((v0_pos + v1_pos) / 2.0).normalize().into();
            let edge_tangent = (v1_pos - v0_pos).normalize().into();
            let radial = midpoint;
            let normal = radial.cross(edge_tangent).normalize();

            let edge_idx = edges.len();
            edge_map.insert(*edge_verts, edge_idx);

            edges.push(Edge {
                vertices: *edge_verts,
                triangles: (tri_list[0] as u32, tri_list[1] as u32),
                midpoint,
                normal,
            });
        }

        let mut cells = Vec::new();
        for tri_idx in 0..num_triangles {
            let base = tri_idx * 3;
            let v0 = indices[base];
            let v1 = indices[base + 1];
            let v2 = indices[base + 2];

            let p0: Vec3 = points[v0 as usize].into();
            let p1: Vec3 = points[v1 as usize].into();
            let p2: Vec3 = points[v2 as usize].into();

            let edge_verts = [
                (v0.min(v1), v0.max(v1)),
                (v1.min(v2), v1.max(v2)),
                (v2.min(v0), v2.max(v0)),
            ];

            let mut cell_edges = [0i32; 3];
            for (i, ev) in edge_verts.iter().enumerate() {
                let edge_idx = (edge_map[ev] + 1) as i32;
                let edge_forward = ev.0 < ev.1;
                let tri_forward = match i {
                    0 => v0 < v1,
                    1 => v1 < v2,
                    2 => v2 < v0,
                    _ => unreachable!(),
                };
                cell_edges[i] = if edge_forward == tri_forward {
                    edge_idx
                } else {
                    -edge_idx
                };
            }

            let e1 = p1 - p0;
            let e2 = p2 - p0;
            let area = e1.cross(e2).length() / 2.0;

            let normal: Vec3 = e1.cross(e2).normalize();
            let grad0 = normal.cross(p2 - p1) / (2.0 * area);
            let grad1 = normal.cross(p0 - p2) / (2.0 * area);
            let grad2 = normal.cross(p1 - p0) / (2.0 * area);

            let center = ((p0 + p1 + p2) / 3.0).normalize();
            let edge_mid0 = (p0 + p1) / 2.0;
            let edge_mid1 = (p1 + p2) / 2.0;
            let edge_mid2 = (p2 + p0) / 2.0;

            let mut edge_normal0 = normal.cross(p1 - p0).normalize();
            let mut edge_normal1 = normal.cross(p2 - p1).normalize();
            let mut edge_normal2 = normal.cross(p0 - p2).normalize();

            if edge_normal0.dot(edge_mid0 - center) < 0.0 {
                edge_normal0 = -edge_normal0;
            }
            if edge_normal1.dot(edge_mid1 - center) < 0.0 {
                edge_normal1 = -edge_normal1;
            }
            if edge_normal2.dot(edge_mid2 - center) < 0.0 {
                edge_normal2 = -edge_normal2;
            }

            cells.push(CellData {
                center,
                pressure: tri_idx as f32,
                flux: vec![0.0; 3],
                vertices: [v0, v1, v2],
                edges: cell_edges,
                barycentric_gradients: [grad0, grad1, grad2],
                edge_normals: [edge_normal0, edge_normal1, edge_normal2],
                area,
            });
        }

        Self {
            sphere,
            cells,
            neighbors,
            vertex_triangles,
            edges,
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
