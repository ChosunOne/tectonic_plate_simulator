use bevy::{
    app::{App, Plugin},
    ecs::{
        query::With,
        schedule::IntoScheduleConfigs,
        system::{Commands, Query, Res},
    },
    log::debug,
    render::{
        Render, RenderApp, RenderStartup, RenderSystems,
        renderer::{RenderDevice, RenderQueue},
    },
};

use crate::{
    components::render::{DepartureBindGroup, SwappableBindGroup},
    plugins::{swappable_bind_group::clear_step, velocity::setup_velocity},
    render::double_buffer::DoubleBuffer,
    resources::{
        departure_info::{DepartureInfo, DepartureInfoSync},
        mantle_grid::MantleGrid,
    },
};

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct DepartureInfoPlugin;

impl Plugin for DepartureInfoPlugin {
    fn build(&self, app: &mut App) {
        let departure_sync = DepartureInfoSync::default();
        app.insert_resource(departure_sync.clone());

        let render_app = app.sub_app_mut(RenderApp);
        render_app.insert_resource(departure_sync);
        render_app.add_systems(RenderStartup, setup_departure_info.after(setup_velocity));
        render_app.add_systems(
            Render,
            sync_departure_to_main
                .in_set(RenderSystems::Cleanup)
                .after(clear_step),
        );
    }
}

pub fn setup_departure_info(
    mut commands: Commands,
    grid: Res<MantleGrid>,
    render_device: Res<RenderDevice>,
) {
    debug!("Setup departure info");

    let num_edges = grid.edge_cell_adjacency().len();

    let departure_data = vec![DepartureInfo::default(); num_edges];

    let departure_buffer =
        DoubleBuffer::new(&render_device, &departure_data, Some("departure_buffer"));

    let mut builder = SwappableBindGroup::builder();
    builder.add_compute_double(departure_buffer);
    let swappable = builder.build(&render_device, Some("departure_bind_group"));

    commands.spawn((swappable, DepartureBindGroup));
}

pub fn sync_departure_to_main(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    departure_query: Query<&SwappableBindGroup, With<DepartureBindGroup>>,
    departure_sync: Res<DepartureInfoSync>,
) {
    let Ok(departure_bg) = departure_query.single() else {
        return;
    };

    let Some(departure_data) = departure_bg.read_back_double_buffer_read::<DepartureInfo>(
        0,
        &render_device,
        &render_queue,
    ) else {
        return;
    };

    if let Ok(mut sync_data) = departure_sync.0.lock() {
        *sync_data = departure_data;
    }
}
