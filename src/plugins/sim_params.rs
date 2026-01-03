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
        render_resource::{BufferInitDescriptor, BufferUsages, ShaderStages},
        renderer::{RenderDevice, RenderQueue},
    },
};
use bytemuck::{Pod, Zeroable};

use crate::{
    components::render::{SimParamsBindGroup, SwappableBindGroup},
    constants::BASE_DT,
    resources::simulation_time::SimulationTime,
};

#[repr(C, align(16))]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct SimParams {
    pub dt: f32,
    _padding: [f32; 3],
}

impl SimParams {
    fn new(dt: f32) -> Self {
        Self {
            dt,
            _padding: [0.0; 3],
        }
    }
}

impl Default for SimParams {
    fn default() -> Self {
        Self {
            dt: BASE_DT,
            _padding: [0.0; 3],
        }
    }
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct SimParamsPlugin;

impl Plugin for SimParamsPlugin {
    fn build(&self, app: &mut App) {
        let sim_time = SimulationTime::new();
        app.insert_resource(sim_time.clone());

        let render_app = app.sub_app_mut(RenderApp);
        render_app.insert_resource(sim_time);
        render_app.add_systems(RenderStartup, setup_sim_params);
        render_app.add_systems(
            Render,
            update_sim_params.in_set(RenderSystems::PrepareResources),
        );
    }
}

pub fn setup_sim_params(mut commands: Commands, render_device: Res<RenderDevice>) {
    debug!("Setup sim params");

    let sim_params = SimParams::default();
    let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("sim_params_buffer"),
        contents: bytemuck::bytes_of(&sim_params),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });

    let mut builder = SwappableBindGroup::builder();
    builder.add_uniform(buffer, ShaderStages::COMPUTE);
    let swappable = builder.build(&render_device, Some("sim_params_bind_group"));

    commands.spawn((swappable, SimParamsBindGroup));
}

pub fn update_sim_params(
    sim_time: Res<SimulationTime>,
    render_queue: Res<RenderQueue>,
    query: Query<&SwappableBindGroup, With<SimParamsBindGroup>>,
) {
    let Ok(bind_group) = query.single() else {
        return;
    };

    let Some(buffer) = bind_group.get_buffer(0) else {
        return;
    };

    let sim_params = SimParams::new(sim_time.dt());

    render_queue.write_buffer(buffer, 0, bytemuck::bytes_of(&sim_params));
}
