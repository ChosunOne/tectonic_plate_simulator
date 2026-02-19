use std::f32::consts::TAU;

use bevy::math::{Vec3, Vec3A};
use sprs::CsMatViewI;

use crate::{constants::SPHERE_RADIUS, resources::mesh_grid::CellData};

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
    /// - `axis_x` points toward the first adjacent edge projected onto the tangent plane.
    /// - `axis_y` is perpendicular to `axis_x` in the tangent plane
    /// # Panics
    /// If there is no adjacent edge to the indicated vertex.
    #[must_use]
    pub fn from_vertex(
        vertex_idx: usize,
        points: &[Vec3A],
        cells: &[CellData],
        vertex_edge_adjacency: CsMatViewI<u32, u32>,
        edge_vertex_adjacency: CsMatViewI<u32, u32>,
        edge_cell_adjacency: CsMatViewI<u32, u32>,
    ) -> Self {
        let origin = Vec3::from(SPHERE_RADIUS * points[vertex_idx]);

        let edge_0_idx = *vertex_edge_adjacency
            .get(vertex_idx, 0)
            .expect("vertex should have at least one adjacent edge")
            as usize;
        let mut frame = Self::from_edge(
            edge_0_idx,
            points,
            cells,
            edge_vertex_adjacency,
            edge_cell_adjacency,
        );
        frame.origin = origin;
        frame
    }

    #[must_use]
    pub fn from_edge(
        edge_idx: usize,
        points: &[Vec3A],
        cells: &[CellData],
        edge_vertex_adjacency: CsMatViewI<u32, u32>,
        edge_cell_adjacency: CsMatViewI<u32, u32>,
    ) -> Self {
        let (v_low, v_high) = Self::get_edge_verts(edge_idx, points, edge_vertex_adjacency);

        let origin = (v_low + v_high) / 2.0;
        let edge_dir = (v_high - v_low).normalize();
        let axis_x = -edge_dir;
        let surface_normal = origin.normalize();
        let perp = surface_normal.cross(edge_dir).normalize();

        let edge_cells = edge_cell_adjacency
            .outer_view(edge_idx)
            .expect("to have cells for edge")
            .iter()
            .map(|(_, &x)| x as usize)
            .collect::<Vec<_>>();
        let primary_cell_center = cells[edge_cells[0]].center;
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

    fn get_edge_verts(
        edge_idx: usize,
        points: &[Vec3A],
        edge_vertex_adjacency: CsMatViewI<u32, u32>,
    ) -> (Vec3, Vec3) {
        let edge_verts = edge_vertex_adjacency
            .outer_view(edge_idx)
            .expect("to have vertices for edge")
            .iter()
            .map(|(_, &x)| x as usize)
            .collect::<Vec<_>>();
        let v_low = Vec3::from(points[edge_verts[0]]);
        let v_high = Vec3::from(points[edge_verts[1]]);

        (v_low, v_high)
    }

    /// Converts a compass bearing (in radians) to a local angle in this frame.
    /// At the poles where north is undefined, returns 0.0
    #[must_use]
    pub fn bearing_to_local_angle(&self, bearing: f32) -> f32 {
        let normal = self.origin.normalize();
        let north_pole = Vec3::Y;

        let toward_north_proj = north_pole - normal * normal.dot(north_pole);
        let toward_north_proj_len = toward_north_proj.length();

        if toward_north_proj_len < f32::EPSILON {
            return 0.0;
        }

        let toward_north = toward_north_proj / toward_north_proj_len;
        let toward_east = toward_north.cross(normal);

        let world_dir = bearing.cos() * toward_north + bearing.sin() * toward_east;

        let local_x = world_dir.dot(self.axis_x);
        let local_y = world_dir.dot(self.axis_y);

        (local_y.atan2(local_x) + TAU) % TAU
    }
}
