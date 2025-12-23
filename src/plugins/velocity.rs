use bevy::{
    app::{App, Plugin},
    ecs::{
        query::With,
        schedule::IntoScheduleConfigs,
        system::{Commands, Query, Res},
    },
    render::{
        Render, RenderApp, RenderStartup, RenderSystems,
        renderer::{RenderDevice, RenderQueue},
    },
};

use crate::{
    components::render::{SwappableBindGroup, VelocityBindGroup},
    plugins::swappable_bind_group::swap_bind_groups,
    render::double_buffer::DoubleBuffer,
    resources::{mantle_grid::MantleGrid, velocity::VelocitySync},
};

pub struct VelocityPlugin;

impl Plugin for VelocityPlugin {
    fn build(&self, app: &mut App) {
        let velocity_sync = VelocitySync::default();
        app.insert_resource(velocity_sync.clone());
        let render_app = app.sub_app_mut(RenderApp);
        render_app.insert_resource(velocity_sync);
        render_app.add_systems(RenderStartup, setup_velocity);
        render_app.add_systems(
            Render,
            sync_velocity_to_main
                .in_set(RenderSystems::Cleanup)
                .after(swap_bind_groups),
        );
    }
}

fn setup_velocity(mut commands: Commands, grid: Res<MantleGrid>, render_device: Res<RenderDevice>) {
    let num_edges = grid.edge_cell_adjacency().len();
    let mut velocity_data = Vec::with_capacity(num_edges);
    for _ in 0..num_edges {
        let angle = 0.5f32;
        let magnitude = 500.0;
        velocity_data.push([magnitude, angle]);
    }

    let velocity_buffer =
        DoubleBuffer::new(&render_device, &velocity_data, Some("velocity_buffer"));

    let mut builder = SwappableBindGroup::builder();
    builder.add_compute_double(velocity_buffer);
    let swappable = builder.build(&render_device, Some("velocity_bind_group"));

    commands.spawn((swappable, VelocityBindGroup));
}

fn sync_velocity_to_main(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    velocity_query: Query<&SwappableBindGroup, With<VelocityBindGroup>>,
    velocity_sync: Res<VelocitySync>,
) {
    let Ok(velocity_bg) = velocity_query.single() else {
        return;
    };

    let Some(velocities) =
        velocity_bg.read_back_double_buffer_read::<[f32; 2]>(0, &render_device, &render_queue)
    else {
        return;
    };

    if let Ok(mut sync_data) = velocity_sync.0.lock() {
        *sync_data = velocities;
    }
}
