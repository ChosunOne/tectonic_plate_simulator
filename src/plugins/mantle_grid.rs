use bevy::{
    app::{App, Plugin},
    render::RenderApp,
};

use crate::resources::mantle_grid::MantleGrid;

pub struct MantleGridPlugin;

impl Plugin for MantleGridPlugin {
    fn build(&self, app: &mut App) {
        let grid = MantleGrid::new(20);
        app.insert_resource(grid.clone());
        let render_app = app.sub_app_mut(RenderApp);
        render_app.insert_resource(grid);
    }
}
