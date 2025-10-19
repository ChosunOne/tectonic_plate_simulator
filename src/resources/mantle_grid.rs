use std::collections::HashMap;

use bevy::{prelude::*, render::extract_resource::ExtractResource};
use hexasphere::shapes::IcoSphere;

#[derive(Resource, Clone)]
pub struct MantleGrid {
    pub sphere: IcoSphere<()>,
    pub cells: Vec<CellData>,
    pub neighbors: Vec<Vec<usize>>,
}

impl ExtractResource for MantleGrid {
    type Source = MantleGrid;

    fn extract_resource(source: &Self::Source) -> Self {
        source.clone()
    }
}

impl MantleGrid {
    #[must_use]
    pub fn new(subdivisions: usize) -> Self {
        let sphere = IcoSphere::new(subdivisions, |_| {});
        let indices = sphere.get_all_indices();
        let num_triangles = indices.len() / 3;
        let cells = (0..num_triangles)
            .map(|x| CellData {
                pressure: x as f32,
                center: triangle_center(&sphere, x),
                flux: vec![0.0; 3],
            })
            .collect();

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

        Self {
            sphere,
            cells,
            neighbors,
        }
    }
}

fn triangle_center(sphere: &IcoSphere<()>, triangle_idx: usize) -> Vec3 {
    let indices = sphere.get_all_indices();
    let base = triangle_idx * 3;
    let points = sphere.raw_points();

    let a = points[indices[base] as usize];
    let b = points[indices[base + 1] as usize];
    let c = points[indices[base + 2] as usize];

    ((a + b + c) / 3.0).normalize().into()
}

#[derive(Debug, Clone)]
pub struct CellData {
    pub center: Vec3,
    pub flux: Vec<f32>,
    pub pressure: f32,
}
