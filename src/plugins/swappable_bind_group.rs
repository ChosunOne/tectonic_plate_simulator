use bevy::{
    app::Plugin,
    ecs::{schedule::IntoScheduleConfigs, system::Query},
    log::debug,
    render::{Render, RenderApp, RenderSystems},
};

use crate::components::render::swappable_bind_group::SwappableBindGroup;

pub struct SwappableBindGroupPlugin;

impl Plugin for SwappableBindGroupPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        debug!("Setup swappable bind groups");
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(Render, swap_bind_groups.in_set(RenderSystems::Cleanup));
    }
}

pub fn swap_bind_groups(mut query: Query<&mut SwappableBindGroup>) {
    for mut bind_group in &mut query {
        bind_group.swap();
    }
}
