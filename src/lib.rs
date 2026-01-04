use bevy::math::Vec3;

use crate::{constants::SPHERE_RADIUS, resources::mantle_grid::MantleGrid};

pub mod components;
pub mod constants;
pub mod materials;
pub mod plugins;
pub mod render;
pub mod resources;
pub mod systems;

#[derive(Debug, Default, Copy, Clone)]
pub struct LocalFrame {
    pub origin: Vec3,
    pub axis_x: Vec3,
    pub axis_y: Vec3,
}

impl LocalFrame {
    /// Create a local frame centered on a vertex.
    /// - axis_x points toward the first adjacent vertex projected onto the tangent plane.
    /// - axis_y is perpendicular to axis_x in the tangent plane
    /// # Panics
    /// If there is no adjacent edge to the indicated vertex.
    #[must_use]
    pub fn from_vertex(grid: &MantleGrid, vertex_idx: usize) -> Self {
        let points = grid.sphere().raw_points();
        let origin = Vec3::from(SPHERE_RADIUS * points[vertex_idx]);
        let normal = origin.normalize();

        let edge_0_idx = grid
            .vertex_edge_adjacency()
            .get(vertex_idx)
            .next()
            .expect("vertex should have at least one adjacent edge");
        let edge_0_verts = grid
            .edge_vertex_adjacency()
            .get(edge_0_idx)
            .collect::<Vec<_>>();
        let v_other = if edge_0_verts[0] == vertex_idx {
            edge_0_verts[1]
        } else {
            edge_0_verts[0]
        };

        let other_pos = Vec3::from(SPHERE_RADIUS * points[v_other]);
        let toward_self = (origin - other_pos).normalize();
        let axis_x = (toward_self - normal * toward_self.dot(normal)).normalize();
        let axis_y = axis_x.cross(normal).normalize();

        Self {
            origin,
            axis_x,
            axis_y,
        }
    }

    #[must_use]
    pub fn from_edge(grid: &MantleGrid, edge_idx: usize) -> Self {
        let (v_low, v_high) = Self::get_edge_verts(grid, edge_idx);

        let origin = (v_low + v_high) / 2.0;
        let edge_dir = (v_high - v_low).normalize();
        let axis_x = -edge_dir;
        let surface_normal = origin.normalize();
        let perp = surface_normal.cross(edge_dir).normalize();

        let edge_cells = grid.edge_cell_adjacency().get(edge_idx).collect::<Vec<_>>();
        let primary_cell_center = grid.cells()[edge_cells[0]].center;
        let to_primary = primary_cell_center - origin;
        let axis_y = if to_primary.dot(perp) > 0.0 {
            perp
        } else {
            -perp
        };

        Self {
            origin,
            axis_x,
            axis_y,
        }
    }

    /// Projects a polar vector in a local edge reference frame to world
    /// coordinates.
    #[must_use]
    pub fn polar_to_world_position(&self, magnitude: f32, angle: f32) -> Vec3 {
        let local_x = magnitude * angle.cos();
        let local_y = magnitude * angle.sin();
        self.origin + local_x * self.axis_x + local_y * self.axis_y
    }

    fn get_edge_verts(grid: &MantleGrid, edge_idx: usize) -> (Vec3, Vec3) {
        let points = grid.sphere().raw_points();
        let edge_verts = grid
            .edge_vertex_adjacency()
            .get(edge_idx)
            .collect::<Vec<_>>();
        let v_low = Vec3::from(points[edge_verts[0]] * SPHERE_RADIUS);
        let v_high = Vec3::from(points[edge_verts[1]] * SPHERE_RADIUS);

        (v_low, v_high)
    }
}
