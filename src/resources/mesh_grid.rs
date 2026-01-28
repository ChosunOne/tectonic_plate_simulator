use std::{collections::HashSet, f64::consts::TAU, sync::Arc};

use bevy::{
    asset::RenderAssetUsages,
    math::DVec3,
    mesh::{Indices, PrimitiveTopology},
    platform::collections::HashMap,
    prelude::*,
    render::extract_resource::ExtractResource,
};

use hexasphere::shapes::IcoSphere;
use sparse::solver::MinNormSolver;
use sprs::{CsMat, CsMatI, CsMatView, CsMatViewI, TriMat, TriMatI};

use crate::constants::SPHERE_RADIUS;

const MAX_EDGES_PER_VERTEX: usize = 6;

#[derive(Debug, Clone)]
pub struct CellData {
    pub center: Vec3,
    pub vertices: [u32; 3],
}

// NB: Structured this way to allow fast sharing between render and main world
#[derive(Resource, Clone)]
pub struct MeshGrid(Arc<MeshGridInner>);

impl MeshGrid {
    #[must_use]
    pub fn new(subdivisions: usize) -> Self {
        Self(Arc::new(MeshGridInner::new(subdivisions)))
    }

    #[must_use]
    pub fn mesh(&self) -> Mesh {
        self.0.mesh()
    }

    #[must_use]
    pub fn sphere(&self) -> &IcoSphere<Vec3A> {
        &self.0.sphere
    }

    #[must_use]
    pub fn cells(&self) -> &[CellData] {
        &self.0.cells
    }

    #[must_use]
    pub fn cell_adjacency(&self) -> CsMatViewI<u32, u32> {
        self.0.cell_adjacency.view()
    }

    #[must_use]
    pub fn cell_edge_adjacency(&self) -> CsMatViewI<u32, u32> {
        self.0.cell_edge_adjacency.view()
    }

    #[must_use]
    pub fn edge_cell_adjacency(&self) -> CsMatViewI<u32, u32> {
        self.0.edge_cell_adjacency.view()
    }

    #[must_use]
    pub fn edge_vertex_adjacency(&self) -> CsMatViewI<u32, u32> {
        self.0.edge_vertex_adjacency.view()
    }

    #[must_use]
    pub fn vertex_cell_adjacency(&self) -> CsMatViewI<u32, u32> {
        self.0.vertex_cell_adjacency.view()
    }

    #[must_use]
    pub fn vertex_edge_adjacency(&self) -> CsMatViewI<u32, u32> {
        self.0.vertex_edge_adjacency.view()
    }

    #[must_use]
    pub fn vertex_angle_offsets(&self) -> &[f32] {
        &self.0.vertex_angle_offsets
    }

    #[must_use]
    pub fn edge_transport_connection(&self) -> &[f32] {
        &self.0.edge_transport_connection
    }

    #[must_use]
    pub fn edge_lengths(&self) -> &[f32] {
        &self.0.edge_lengths
    }

    #[must_use]
    pub fn edge_centroid_distance(&self) -> &[f32] {
        &self.0.edge_centroid_distance
    }
}

fn build_cell_edge_adjacency<T>(sphere: &IcoSphere<T>) -> CsMatI<u32, u32> {
    let mesh_indices = sphere.get_all_indices();
    let num_cells = mesh_indices.len() / 3;

    let mut cell_edge_adjacency = TriMatI::<u32, u32>::new((num_cells, 3));

    let mut edge_idx = 0;
    let mut edge_map = HashMap::new();
    for cell_idx in 0..num_cells {
        let base = cell_idx * 3;
        let cell_verts = [
            mesh_indices[base],
            mesh_indices[base + 1],
            mesh_indices[base + 2],
        ];

        for local_edge_idx in 0..3 {
            let v0 = cell_verts[local_edge_idx];
            let v1 = cell_verts[(local_edge_idx + 1) % 3];
            let canonical_edge = (v0.min(v1), v0.max(v1));
            if !edge_map.contains_key(&canonical_edge) {
                edge_map.insert(canonical_edge, edge_idx);
                edge_idx += 1;
            }
            let i = edge_map[&canonical_edge];

            cell_edge_adjacency.add_triplet(cell_idx, local_edge_idx, i);
        }
    }

    cell_edge_adjacency.to_csr()
}

fn build_cell_adjacency(
    cell_edges: CsMatViewI<u32, u32>,
    edge_cells: CsMatViewI<u32, u32>,
) -> CsMatI<u32, u32> {
    let num_cells = cell_edges.rows();

    let mut cell_adjacency = TriMatI::<u32, u32>::new((num_cells, 3));

    for (cell_idx, edges) in cell_edges.outer_iterator().enumerate() {
        let mut count = 0;
        for (_, &edge_idx) in edges.iter() {
            let cells = [
                *edge_cells
                    .get(edge_idx as usize, 0)
                    .expect("to have cell for edge"),
                *edge_cells
                    .get(edge_idx as usize, 1)
                    .expect("to have cell for edge"),
            ];
            let neighbor = if cells[0] as usize == cell_idx {
                cells[1]
            } else {
                cells[0]
            };
            cell_adjacency.add_triplet(cell_idx, count, neighbor as u32);
            count += 1;
        }
    }

    cell_adjacency.to_csr()
}

fn build_edge_vertex_adjacency<T>(sphere: &IcoSphere<T>) -> CsMatI<u32, u32> {
    let mesh_indices = sphere.get_all_indices();
    let num_cells = mesh_indices.len() / 3;
    let num_edges = 3 * num_cells / 2;

    let mut edge_vertex_adjacency = TriMatI::<u32, u32>::new((num_edges, 2));
    let mut inserted = HashSet::new();

    let mut edge_idx = 0;
    let mut edge_map = HashMap::new();
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

            if !edge_map.contains_key(&canonical) {
                edge_map.insert(canonical, edge_idx);
                edge_idx += 1;
            }

            let i = edge_map[&canonical];
            if inserted.contains(&i) {
                continue;
            }
            inserted.insert(i);

            edge_vertex_adjacency.add_triplet(i, 0, canonical.0);
            edge_vertex_adjacency.add_triplet(i, 1, canonical.1);
        }
    }
    edge_vertex_adjacency.to_csr()
}

fn build_edge_cell_adjacency<T>(sphere: &IcoSphere<T>) -> CsMatI<u32, u32> {
    let mesh_indices = sphere.get_all_indices();
    let num_cells = mesh_indices.len() / 3;
    let num_edges = 3 * num_cells / 2;

    let mut edge_cell_adjacency = TriMatI::<u32, u32>::new((num_edges, 2));
    let mut edge_map = HashMap::new();
    let mut edge_idx = 0;
    let mut inserted = HashSet::new();

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
            let col = usize::from(v0 >= v1);
            let canonical = (v0.min(v1), v0.max(v1));
            if !edge_map.contains_key(&canonical) {
                edge_map.insert(canonical, edge_idx);
                edge_idx += 1;
            }

            let i = edge_map[&canonical];
            if inserted.contains(&(i, col)) {
                continue;
            }
            inserted.insert((i, col));
            edge_cell_adjacency.add_triplet(i, col, cell_idx as u32);
        }
    }

    edge_cell_adjacency.to_csr()
}

fn build_vertex_cell_adjacency<T>(sphere: &IcoSphere<T>) -> CsMatI<u32, u32> {
    let num_vertices = sphere.raw_points().len();
    let mesh_indices = sphere.get_all_indices();
    let num_cells = mesh_indices.len() / 3;

    let mut vertex_cell_adjacency = TriMatI::<u32, u32>::new((num_vertices, num_cells));
    let mut inserted = HashSet::new();

    for cell_idx in 0..num_cells {
        let base = cell_idx * 3;
        let v0 = mesh_indices[base];
        let v1 = mesh_indices[base + 1];
        let v2 = mesh_indices[base + 2];

        if !inserted.contains(&(v0, cell_idx)) {
            vertex_cell_adjacency.add_triplet(v0 as usize, cell_idx, cell_idx as u32);
            inserted.insert((v0, cell_idx));
        }
        if !inserted.contains(&(v1, cell_idx)) {
            vertex_cell_adjacency.add_triplet(v1 as usize, cell_idx, cell_idx as u32);
            inserted.insert((v1, cell_idx));
        }
        if !inserted.contains(&(v2, cell_idx)) {
            vertex_cell_adjacency.add_triplet(v2 as usize, cell_idx, cell_idx as u32);
            inserted.insert((v2, cell_idx));
        }
    }

    vertex_cell_adjacency.to_csr()
}

fn build_vertex_edge_adjacency<T>(
    sphere: &IcoSphere<T>,
    edge_vertices: CsMatViewI<u32, u32>,
) -> CsMatI<u32, u32> {
    let points = sphere.raw_points();
    let num_vertices = points.len();
    let num_edges = edge_vertices.rows();

    let mut vertex_edge_adjacency = TriMatI::<u32, u32>::new((num_vertices, MAX_EDGES_PER_VERTEX));
    let mut vertex_edges = vec![Vec::new(); num_vertices];
    let mut inserted = HashSet::new();

    for edge_idx in 0..num_edges {
        let verts = edge_vertices
            .outer_view(edge_idx)
            .expect("to have vertices for edge")
            .iter()
            .map(|(_, &x)| x as usize)
            .collect::<Vec<_>>();
        let v0 = verts[0];
        let v1 = verts[1];

        vertex_edges[v0 as usize].push(edge_idx as u32);
        vertex_edges[v1 as usize].push(edge_idx as u32);
    }

    for (vertex_idx, edges) in vertex_edges.iter_mut().enumerate() {
        let vertex_pos = Vec3::from(points[vertex_idx]);
        let vertex_normal = vertex_pos.normalize();
        let is_pole = vertex_normal.x.abs() < 1e-6
            && vertex_normal.z.abs() < 1e-6
            && (vertex_normal.y.abs() - 1.0).abs() < 1e-6;

        let up = if is_pole { Vec3::X } else { Vec3::Y };
        let tangent_x = vertex_normal.cross(up).normalize();
        let tangent_y = tangent_x.cross(vertex_normal).normalize();
        let mut edge_angles = edges
            .iter()
            .map(|&edge_idx| {
                let vertices = [
                    *edge_vertices
                        .get(edge_idx as usize, 0)
                        .expect("to have vertex for edge"),
                    *edge_vertices
                        .get(edge_idx as usize, 1)
                        .expect("to have vertex for edge"),
                ];
                let other_vertex = if vertices[0] as usize == vertex_idx {
                    vertices[1] as usize
                } else {
                    vertices[0] as usize
                };
                let other_pos = Vec3::from(points[other_vertex]);
                let direction = (other_pos - vertex_pos).normalize();

                let proj_x = direction.dot(tangent_x);
                let proj_y = direction.dot(tangent_y);
                let angle = proj_y.atan2(proj_x);

                (edge_idx, angle)
            })
            .collect::<Vec<_>>();
        edge_angles.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        *edges = edge_angles.into_iter().map(|(idx, _)| idx).collect();

        for (col, &edge) in edges.iter().enumerate() {
            if inserted.contains(&(vertex_idx, col)) {
                continue;
            }
            vertex_edge_adjacency.add_triplet(vertex_idx, col, edge);
            inserted.insert((vertex_idx, col));
        }
    }

    vertex_edge_adjacency.to_csr()
}

struct MeshGridInner {
    pub cell_adjacency: CsMatI<u32, u32>,
    pub cell_edge_adjacency: CsMatI<u32, u32>,
    pub cells: Vec<CellData>,
    pub edge_cell_adjacency: CsMatI<u32, u32>,
    pub edge_centroid_distance: Vec<f32>,
    pub edge_lengths: Vec<f32>,
    pub edge_transport_connection: Vec<f32>,
    pub edge_vertex_adjacency: CsMatI<u32, u32>,
    pub sphere: IcoSphere<Vec3A>,
    pub vertex_cell_adjacency: CsMatI<u32, u32>,
    pub vertex_edge_adjacency: CsMatI<u32, u32>,
    pub vertex_angle_offsets: Vec<f32>,
}

impl ExtractResource for MeshGrid {
    type Source = MeshGrid;

    fn extract_resource(source: &Self::Source) -> Self {
        source.clone()
    }
}

impl MeshGridInner {
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn new(subdivisions: usize) -> Self {
        let sphere = IcoSphere::new(subdivisions, |v| v * SPHERE_RADIUS);
        let points = sphere.raw_points();
        let indices = sphere.get_all_indices();
        let num_triangles = indices.len() / 3;

        let cell_edge_adjacency = build_cell_edge_adjacency(&sphere);
        let edge_cell_adjacency = build_edge_cell_adjacency(&sphere);
        let cell_adjacency =
            build_cell_adjacency(cell_edge_adjacency.view(), edge_cell_adjacency.view());
        let edge_vertex_adjacency = build_edge_vertex_adjacency(&sphere);
        let vertex_cell_adjacency = build_vertex_cell_adjacency(&sphere);
        let vertex_edge_adjacency =
            build_vertex_edge_adjacency(&sphere, edge_vertex_adjacency.view());
        let vertex_angle_offsets = build_vertex_angle_offsets(
            points,
            vertex_edge_adjacency.view(),
            edge_vertex_adjacency.view(),
        );
        let edge_lengths = build_edge_lenths(
            edge_cell_adjacency.view(),
            edge_vertex_adjacency.view(),
            sphere.raw_points(),
        );
        let edge_centroid_distance = build_edge_centroid_distance(
            cell_edge_adjacency.view(),
            edge_cell_adjacency.view(),
            &edge_lengths,
        );

        let mut cells = Vec::new();
        for tri_idx in 0..num_triangles {
            let base = tri_idx * 3;
            let v0 = indices[base];
            let v1 = indices[base + 1];
            let v2 = indices[base + 2];

            let p0: Vec3 = (SPHERE_RADIUS * points[v0 as usize]).into();
            let p1: Vec3 = (SPHERE_RADIUS * points[v1 as usize]).into();
            let p2: Vec3 = (SPHERE_RADIUS * points[v2 as usize]).into();

            let center = (p0 + p1 + p2) / 3.0;

            cells.push(CellData {
                center,
                vertices: [v0, v1, v2],
            });
        }

        let edge_transport_connection = Self::calculate_trivial_connection(
            cell_edge_adjacency.rows(),
            &[(0, 1), (11, 1)],
            vertex_edge_adjacency.view(),
            edge_cell_adjacency.view(),
            edge_vertex_adjacency.view(),
            &sphere,
        );

        Self {
            cell_adjacency,
            cell_edge_adjacency,
            cells,
            edge_cell_adjacency,
            edge_centroid_distance,
            edge_lengths,
            edge_transport_connection,
            edge_vertex_adjacency,
            sphere,
            vertex_cell_adjacency,
            vertex_edge_adjacency,
            vertex_angle_offsets,
        }
    }

    #[must_use]
    pub fn mesh(&self) -> Mesh {
        let points = self.sphere.raw_points();
        let indices = self.sphere.get_all_indices();

        let positions = points
            .iter()
            .map(|&p| (SPHERE_RADIUS * p).into())
            .collect::<Vec<[f32; 3]>>();
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

    fn calculate_trivial_connection(
        num_faces: usize,
        singularities: &[(usize, usize)],
        vertex_edge_adjacency: CsMatViewI<u32, u32>,
        edge_cell_adjacency: CsMatViewI<u32, u32>,
        edge_vertex_adjacency: CsMatViewI<u32, u32>,
        sphere: &IcoSphere<Vec3A>,
    ) -> Vec<f32> {
        let num_vertices = vertex_edge_adjacency.rows();
        let num_edges = edge_cell_adjacency.rows();
        let euler_characteristic = num_vertices + num_faces - num_edges;
        debug_assert_eq!(
            euler_characteristic,
            singularities.iter().fold(0, |acc, x| { acc + x.1 })
        );
        let d0 = Self::build_d0(vertex_edge_adjacency, edge_cell_adjacency);
        let mut curvature = Self::calculate_gaussian_curvature(
            sphere,
            vertex_edge_adjacency,
            edge_vertex_adjacency,
        );

        for &(vertex_idx, index) in singularities {
            curvature[vertex_idx] -= TAU * (index as f64);
        }

        let mut neg_curvature = vec![0.0; curvature.len()];
        for i in 0..neg_curvature.len() {
            neg_curvature[i] = -curvature[i];
        }

        let a = d0.transpose_view();
        let connection = Self::solve_min_norm(a, &neg_curvature);

        connection
            .into_iter()
            .map(|x| {
                if x.abs() < f64::from(f32::EPSILON) {
                    0.0
                } else {
                    x as f32
                }
            })
            .collect()
    }

    fn solve_min_norm(a: CsMatView<f64>, b: &[f64]) -> Vec<f64> {
        let mut solver = MinNormSolver::new().expect("to create solver");
        solver.solve_min_norm(a, b).expect("to solve min norm")
    }

    fn calculate_gaussian_curvature(
        sphere: &IcoSphere<Vec3A>,
        vertex_edge_adjacency: CsMatViewI<u32, u32>,
        edge_vertex_adjacency: CsMatViewI<u32, u32>,
    ) -> Vec<f64> {
        let mut curvature = vec![0.0; vertex_edge_adjacency.rows()];
        for i in 0..vertex_edge_adjacency.rows() {
            let mut angle_sum = 0.0;
            let mut prev_edge_dir = DVec3::default();
            let mut adjacent_edges = vertex_edge_adjacency
                .outer_view(i)
                .expect("to have edges for vertex")
                .iter()
                .map(|(_, &x)| x)
                .collect::<Vec<_>>();
            adjacent_edges.push(adjacent_edges[0]);
            for (e_i, &edge) in adjacent_edges.iter().enumerate() {
                let verts = edge_vertex_adjacency
                    .outer_view(edge as usize)
                    .expect("to have vertices for edge")
                    .iter()
                    .map(|(_, &x)| x)
                    .collect::<Vec<_>>();
                let other_vertex = {
                    if verts[0] as usize == i {
                        verts[1] as usize
                    } else {
                        verts[0] as usize
                    }
                };

                let dir = sphere.raw_points()[other_vertex] - sphere.raw_points()[i];
                let dir_64 = DVec3::from(Vec3::from(dir));

                if e_i != 0 {
                    angle_sum += prev_edge_dir.angle_between(dir_64);
                }

                prev_edge_dir = dir_64;
            }

            curvature[i] = TAU - angle_sum;
        }
        curvature
    }

    fn build_d0(
        vertex_edge_adjacency: CsMatViewI<u32, u32>,
        edge_cell_adjacency: CsMatViewI<u32, u32>,
    ) -> CsMat<f64> {
        let num_vertices = vertex_edge_adjacency.rows();
        let mut d0_triplets = TriMat::new((edge_cell_adjacency.rows(), num_vertices));

        for vertex_idx in 0..num_vertices {
            let adjacent_edges = vertex_edge_adjacency
                .outer_view(vertex_idx as usize)
                .expect("to have edges for vertex")
                .iter()
                .map(|(_, &x)| x)
                .collect::<Vec<_>>();
            for i in 0..adjacent_edges.len() {
                let edge_idx = adjacent_edges[i];
                let last_edge_idx = adjacent_edges
                    [((i as isize - 1).rem_euclid(adjacent_edges.len() as isize)) as usize];
                let last_cells = edge_cell_adjacency
                    .outer_view(last_edge_idx as usize)
                    .expect("to have cells for edge")
                    .iter()
                    .map(|(_, &x)| x)
                    .collect::<Vec<_>>();
                let cells = edge_cell_adjacency
                    .outer_view(edge_idx as usize)
                    .expect("to have cells for edge")
                    .iter()
                    .map(|(_, &x)| x)
                    .collect::<Vec<_>>();

                // find the overlapping cell and establish the direction in which we are traversing the cells
                let overlapping_cell_idx = *cells.iter().find(|&c| last_cells.contains(c)).unwrap();
                // From secondary -> primary +1
                // From primary -> secondary -1
                let sign = if overlapping_cell_idx == cells[0] {
                    -1.0
                } else {
                    1.0
                };

                d0_triplets.add_triplet(edge_idx as usize, vertex_idx, sign);
            }
        }

        d0_triplets.to_csc()
    }
}

fn build_vertex_angle_offsets(
    points: &[Vec3A],
    vertex_edge_adjacency: CsMatViewI<u32, u32>,
    edge_vertex_adjacency: CsMatViewI<u32, u32>,
) -> Vec<f32> {
    let num_vertices = points.len();
    let mut vertex_angle_offsets = vec![0.0f32; num_vertices];
    let mut pole_vertices = Vec::new();
    for vertex_idx in 0..num_vertices {
        let vertex_pos: Vec3 = points[vertex_idx].into();
        let vertex_normal = vertex_pos.normalize();

        let is_pole = vertex_normal.x.abs() < 1e-7
            && vertex_normal.z.abs() < 1e-7
            && (vertex_normal.y.abs() - SPHERE_RADIUS).abs() < 1e-7;

        if is_pole {
            pole_vertices.push(vertex_idx);
            continue;
        }

        let edge_0_idx = vertex_edge_adjacency
            .outer_view(vertex_idx)
            .expect("there to be an edge")
            .iter()
            .map(|(_, &x)| x as usize)
            .next()
            .expect("there to be an edge");

        let edge_0_verts = edge_vertex_adjacency
            .outer_view(edge_0_idx as usize)
            .expect("to have vertices for edge")
            .iter()
            .map(|(_, &x)| x)
            .collect::<Vec<_>>();
        let v_other = if edge_0_verts[0] as usize == vertex_idx {
            edge_0_verts[1] as usize
        } else {
            edge_0_verts[0] as usize
        };
        let other_pos: Vec3 = points[v_other].into();
        let edge_dir = (other_pos - vertex_pos).normalize();

        let edge_dir_tangent = (edge_dir - vertex_normal * edge_dir.dot(vertex_normal)).normalize();

        let west_raw = vertex_normal.cross(Vec3::Y);
        if west_raw.length() < 0.05 * SPHERE_RADIUS {
            pole_vertices.push(vertex_idx);
            continue;
        }

        let west = west_raw.normalize();
        let north = west.cross(vertex_normal).normalize();
        let angle_offset = edge_dir_tangent
            .dot(north)
            .atan2(edge_dir_tangent.dot(west));

        vertex_angle_offsets[vertex_idx] = angle_offset;
    }

    for &pole_idx in &pole_vertices {
        let pole_pos: Vec3 = points[pole_idx].into();
        let pole_normal = pole_pos.normalize();

        let edge_0_idx = vertex_edge_adjacency
            .outer_view(pole_idx)
            .expect("to have pole vertex edge")
            .iter()
            .map(|(_, &x)| x as usize)
            .next()
            .expect("to have pole vertex edge");
        let edge_0_verts = edge_vertex_adjacency
            .outer_view(edge_0_idx as usize)
            .expect("to have vertices for edge")
            .iter()
            .map(|(_, &x)| x)
            .collect::<Vec<_>>();
        let neighbor_idx = if edge_0_verts[0] as usize == pole_idx {
            edge_0_verts[1] as usize
        } else {
            edge_0_verts[0] as usize
        };

        let neighbor_pos: Vec3 = points[neighbor_idx].into();
        let neighbor_normal = neighbor_pos.normalize();
        let neighbor_west = neighbor_normal.cross(Vec3::Y).normalize();
        let neighbor_north = neighbor_west.cross(neighbor_normal).normalize();

        let edge_dir = (neighbor_pos - pole_pos).normalize();
        let edge_dir_tangent = (edge_dir - pole_normal * edge_dir.dot(pole_normal)).normalize();

        let neighbor_west_at_pole =
            (neighbor_west - pole_normal * neighbor_west.dot(pole_normal)).normalize();
        let neighbor_north_at_pole =
            (neighbor_north - pole_normal * neighbor_north.dot(pole_normal)).normalize();

        let angle_offset = edge_dir_tangent
            .dot(neighbor_north_at_pole)
            .atan2(edge_dir_tangent.dot(neighbor_west_at_pole));

        vertex_angle_offsets[pole_idx] = angle_offset;
    }
    vertex_angle_offsets
}

fn build_edge_lenths(
    edge_cell_adjacency: CsMatViewI<u32, u32>,
    edge_vertex_adjacency: CsMatViewI<u32, u32>,
    points: &[Vec3A],
) -> Vec<f32> {
    let mut edge_lengths = vec![0.0f32; edge_cell_adjacency.rows()];
    for (i, length) in edge_lengths.iter_mut().enumerate() {
        let left_vertex_idx = edge_vertex_adjacency.data()[i * 2] as usize;
        let right_vertex_idx = edge_vertex_adjacency.data()[i * 2 + 1] as usize;
        let left_vertex = points[left_vertex_idx] * SPHERE_RADIUS;
        let right_vertex = points[right_vertex_idx] * SPHERE_RADIUS;
        *length = left_vertex.distance(right_vertex);
    }
    edge_lengths
}

fn build_edge_centroid_distance(
    cell_edge_adjacency: CsMatViewI<u32, u32>,
    edge_cell_adjacency: CsMatViewI<u32, u32>,
    edge_lengths: &[f32],
) -> Vec<f32> {
    let mut edge_centroid_distance = vec![0.0f32; edge_cell_adjacency.rows()];
    for (i, distance) in edge_centroid_distance.iter_mut().enumerate() {
        let cells = edge_cell_adjacency
            .outer_view(i)
            .expect("to have cells for edge")
            .iter()
            .map(|(_, &x)| x as usize)
            .collect::<Vec<_>>();
        let primary_cell = cells[0];
        let secondary_cell = cells[1];
        let primary_edges = cell_edge_adjacency
            .outer_view(primary_cell)
            .expect("to have edges for cell")
            .iter()
            .map(|(_, &x)| x as usize)
            .collect::<Vec<_>>();
        let secondary_edges = cell_edge_adjacency
            .outer_view(secondary_cell)
            .expect("to have edges for cell")
            .iter()
            .map(|(_, &x)| x as usize)
            .collect::<Vec<_>>();

        let primary_area = cell_area(&primary_edges, &edge_lengths);
        let secondary_area = cell_area(&secondary_edges, &edge_lengths);

        let h1 = 2.0 * primary_area / edge_lengths[i];
        let h2 = 2.0 * secondary_area / edge_lengths[i];
        *distance = (h1 + h2) / 3.0;
    }

    edge_centroid_distance
}

fn cell_area(edges: &[usize], edge_lengths: &[f32]) -> f32 {
    let a = edge_lengths[edges[0]];
    let b = edge_lengths[edges[1]];
    let c = edge_lengths[edges[2]];

    let s = (a + b + c) / 2.0;
    (s * (s - a) * (s - b) * (s - c)).sqrt()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_calculates_curvature() {
        let grid = MeshGrid::new(20);
        let curvature = MeshGridInner::calculate_gaussian_curvature(
            grid.sphere(),
            grid.vertex_edge_adjacency(),
            grid.edge_vertex_adjacency(),
        );

        let total_curvature = curvature.iter().fold(0.0, |acc, &x| acc + x);

        assert!(
            (total_curvature - 2.0 * TAU).abs() < 1.5e-3,
            "{}",
            (total_curvature - 2.0 * TAU).abs()
        );
    }

    #[test]
    fn it_sums_to_zero_for_d0() {
        let grid = MeshGrid::new(20);

        let d0 = MeshGridInner::build_d0(grid.vertex_edge_adjacency(), grid.edge_cell_adjacency())
            .to_csr();

        for row_vec in d0.outer_iterator() {
            let sum = row_vec.iter().fold(0.0, |acc, (_, &x)| acc + x);
            assert!(sum.abs() < f64::EPSILON, "{sum}");
        }
    }

    #[test]
    fn it_calculates_a_trivial_connection() {
        let grid = MeshGrid::new(100);
        let singularities = &[(0, 1), (11, 1)];
        let connection = MeshGridInner::calculate_trivial_connection(
            grid.cell_edge_adjacency().rows(),
            singularities,
            grid.vertex_edge_adjacency(),
            grid.edge_cell_adjacency(),
            grid.edge_vertex_adjacency(),
            grid.sphere(),
        );

        let mut curvature = MeshGridInner::calculate_gaussian_curvature(
            grid.sphere(),
            grid.vertex_edge_adjacency(),
            grid.edge_vertex_adjacency(),
        );
        for &(edge_idx, k) in singularities {
            curvature[edge_idx] -= TAU * (k as f64);
        }

        let num_vertices = grid.vertex_edge_adjacency().rows();

        for vertex_idx in 0..num_vertices {
            let edges = grid
                .vertex_edge_adjacency()
                .outer_view(vertex_idx)
                .expect("to have edges for vertex")
                .iter()
                .map(|(_, &x)| x as usize)
                .collect::<Vec<_>>();

            let mut signed_sum = 0.0;

            for i in 0..edges.len() {
                let edge_idx = edges[i];
                let prev_edge_idx = edges[(i + edges.len() - 1) % edges.len()];

                let cells = grid
                    .edge_cell_adjacency()
                    .outer_view(edge_idx)
                    .expect("to have cells for edge")
                    .iter()
                    .map(|(_, &x)| x as usize)
                    .collect::<Vec<_>>();
                let prev_cells = grid
                    .edge_cell_adjacency()
                    .outer_view(prev_edge_idx)
                    .expect("to have cells for edge")
                    .iter()
                    .map(|(_, &x)| x as usize)
                    .collect::<Vec<_>>();

                // Find overlapping cell between current and previous edge
                let overlapping = cells.iter().find(|c| prev_cells.contains(c)).unwrap();

                // Canonical orientation: secondary -> primary = cells[1] -> cells[0]
                // Sign is +1 if we traverse from secondary (cells[1]), -1 if from primary (cells[0])
                let sign = if overlapping == &cells[1] { 1.0 } else { -1.0 };

                signed_sum += sign * (connection[edge_idx] as f64);
            }

            // The constraint: sum(sign * x) = -˜K
            let expected = -curvature[vertex_idx];
            assert!(
                (signed_sum - expected).abs() < 1e-6,
                "Vertex {vertex_idx}: signed_sum = {signed_sum}, expected = {expected}",
            );
        }
    }

    fn mod_tau(theta: f32) -> f32 {
        if (0.0..std::f32::consts::TAU).contains(&theta) {
            return theta;
        }
        (theta + std::f32::consts::TAU) % std::f32::consts::TAU
    }

    fn compute_angles(base: usize, left: usize, right: usize, edge_lengths: &[f32]) -> [f32; 3] {
        let a = edge_lengths[base as usize];
        let b = edge_lengths[left as usize];
        let c = edge_lengths[right as usize];

        let a_squared = a * a;
        let b_squared = b * b;
        let c_squared = c * c;

        let left_base_angle = ((a_squared + b_squared - c_squared) / (2.0 * a * b))
            .clamp(-1.0, 1.0)
            .acos();
        let right_base_angle = ((a_squared + c_squared - b_squared) / (2.0 * a * c))
            .clamp(-1.0, 1.0)
            .acos();
        let apex_angle = ((b_squared + c_squared - a_squared) / (2.0 * b * c))
            .clamp(-1.0, 1.0)
            .acos();

        [left_base_angle, right_base_angle, apex_angle]
    }

    fn get_adjacent_edges(
        edge: usize,
        cell: usize,
        edge_cell_indices: &[u32],
        edge_vertex_indices: &[u32],
        cell_edge_indices: &[u32],
    ) -> [usize; 2] {
        let mut left_edge = 0;
        let mut right_edge = 0;
        let is_secondary = cell == edge_cell_indices[edge * 2 + 1] as usize;

        let mut left_vertex = edge_vertex_indices[edge * 2] as usize;
        let mut right_vertex = edge_vertex_indices[edge * 2 + 1] as usize;
        if is_secondary {
            left_vertex = left_vertex ^ right_vertex;
            right_vertex = left_vertex ^ right_vertex;
            left_vertex = left_vertex ^ right_vertex;
        }

        for i in 0..3 {
            let other_edge = cell_edge_indices[cell * 3 + i] as usize;
            if other_edge == edge {
                continue;
            }
            let other_left_vertex = edge_vertex_indices[other_edge * 2] as usize;
            let other_right_vertex = edge_vertex_indices[other_edge * 2 + 1] as usize;
            if other_left_vertex == left_vertex || other_right_vertex == left_vertex {
                left_edge = other_edge;
            } else if other_left_vertex == right_vertex || other_right_vertex == right_vertex {
                right_edge = other_edge;
            }
        }

        [left_edge, right_edge]
    }

    #[test]
    fn it_has_no_holonomy() {
        let grid = MeshGrid::new(5);
        // Path around the equator of the mesh
        let path = vec![
            712, 710, 369, 370, 372, 395, 398, 376, 375, 384, 633, 634, 661, 659, 312, 313, 315,
            338, 341, 319, 318, 327, 582, 583, 610, 608, 540, 541, 543, 566, 569, 547, 546, 555,
            786, 787, 814, 812, 483, 484, 486, 509, 512, 490, 489, 498, 735, 736, 763, 761, 426,
            427, 429, 452, 455, 433, 432, 441, 684, 685, 712,
        ];

        let mut last_cell = *grid.edge_cell_adjacency().get(path[0], 0).unwrap() as usize;
        let mut last_edge = path[0];
        let mut vector_direction = 1.0;
        let initial_vector_direction = vector_direction;
        for &edge in &path[1..] {
            let adjacent_edges = get_adjacent_edges(
                last_edge,
                last_cell,
                grid.edge_cell_adjacency().data(),
                grid.edge_vertex_adjacency().data(),
                grid.cell_edge_adjacency().data(),
            );
            let cells = grid
                .edge_cell_adjacency()
                .outer_view(edge)
                .expect("to have cells for edge")
                .iter()
                .map(|(_, &x)| x as usize)
                .collect::<Vec<_>>();
            let sign = if cells[0] == last_cell {
                last_cell = cells[1];
                -1.0
            } else {
                last_cell = cells[0];
                1.0
            };
            let angles = compute_angles(
                last_edge,
                adjacent_edges[0],
                adjacent_edges[1],
                grid.edge_lengths(),
            );
            let is_left = edge == adjacent_edges[0];
            let geometric_transport = if is_left { angles[0] } else { -angles[1] };
            let parallel_transport = sign * grid.edge_transport_connection()[edge];
            vector_direction += parallel_transport + geometric_transport;
            vector_direction = mod_tau(vector_direction);
            last_edge = edge;
        }
        assert!(
            (vector_direction - initial_vector_direction).abs() < 1e-5,
            "{vector_direction}"
        );
    }
}
