use bevy::{
    app::Plugin,
    ecs::{schedule::IntoScheduleConfigs, system::ResMut},
    render::{Render, RenderApp, RenderSystems},
};

use crate::render::swappable_bind_group::SwappableBindGroup;

pub struct SwappableBindGroupPlugin;

impl Plugin for SwappableBindGroupPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(Render, swap_bind_groups.in_set(RenderSystems::Cleanup));
    }
}

fn swap_bind_groups(bind_group: Option<ResMut<SwappableBindGroup>>) {
    if let Some(mut bind_group) = bind_group {
        bind_group.swap();
    }
}
