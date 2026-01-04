use bevy::{
    app::Plugin,
    ecs::{
        schedule::IntoScheduleConfigs,
        system::{Query, Res},
    },
    log::debug,
    render::{Render, RenderApp, RenderSystems},
};

use crate::{
    components::render::swappable_bind_group::SwappableBindGroup,
    resources::simulation_time::SimulationTime,
};

pub struct SwappableBindGroupPlugin;

impl Plugin for SwappableBindGroupPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        debug!("Setup swappable bind groups");
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(
            Render,
            (swap_bind_groups, clear_step)
                .chain()
                .in_set(RenderSystems::Cleanup),
        );
    }
}

pub fn swap_bind_groups(
    mut query: Query<&mut SwappableBindGroup>,
    sim_time: Option<Res<SimulationTime>>,
) {
    if let Some(st) = sim_time
        && !st.should_run()
    {
        return;
    }
    for mut bind_group in &mut query {
        bind_group.swap();
    }
}

pub fn clear_step(sim_time: Option<Res<SimulationTime>>) {
    if let Some(st) = sim_time {
        st.clear_step();
    }
}
