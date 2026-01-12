use std::f32::consts::PI;

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
    LocalFrame,
    components::render::{SwappableBindGroup, VelocityBindGroup},
    plugins::swappable_bind_group::clear_step,
    render::double_buffer::DoubleBuffer,
    resources::{mesh_grid::MeshGrid, velocity::VelocitySync},
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
                .after(clear_step),
        );
    }
}

pub fn setup_velocity(
    mut commands: Commands,
    grid: Res<MeshGrid>,
    render_device: Res<RenderDevice>,
) {
    debug!("Setup velocity");
    let num_edges = grid.edge_cell_adjacency().len();
    let mut velocity_data = Vec::<[f32; 2]>::with_capacity(num_edges);
    for i in 0..num_edges {
        let frame = LocalFrame::from_edge(&grid, i);
        let latitude = (frame.origin.y / frame.origin.length()).asin();

        let angle = frame.bearing_to_local_angle(PI / 2.0);
        // } else {
        //     frame.bearing_to_local_angle(3.0 * PI / 2.0)
        // };
        let magnitude = 100.0 * latitude.cos();

        // let magnitude;
        // let angle;
        // match i {
        //     2785 => {
        //         magnitude = 17.09977341;
        //         angle = 6.28318501;
        //     }
        //     2784 => {
        //         magnitude = 16.71239471;
        //         angle = 0.92208433;
        //     }
        //     2786 => {
        //         magnitude = 16.72800827;
        //         angle = 5.33526325;
        //     }
        //     1940 => {
        //         magnitude = 15.19938850;
        //         angle = 5.30937481;
        //     }
        //     2793 => {
        //         magnitude = 13.33704472;
        //         angle = 1.00216007;
        //     }
        //     _ => {
        //         magnitude = 0.0;
        //         angle = 0.0;
        //     }
        // }

        velocity_data.push([magnitude, angle]);
    }

    let velocity_buffer =
        DoubleBuffer::new(&render_device, &velocity_data, Some("velocity_buffer"));

    let mut builder = SwappableBindGroup::builder();
    builder.add_compute_double(velocity_buffer);
    let swappable = builder.build(&render_device, "velocity_bind_group");

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
