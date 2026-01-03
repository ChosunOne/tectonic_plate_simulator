use bevy::{
    color::Color,
    ecs::{
        query::With,
        system::{Commands, Query, Res},
    },
    text::{TextColor, TextFont},
    ui::{Node, PositionType, Val, widget::Text},
};

use crate::{
    components::ui::simulation::SimulationStatusText, resources::simulation_time::SimulationTime,
};

pub fn setup_simulation_ui(mut commands: Commands) {
    commands.spawn((
        Text::new("Speed: 1x"),
        TextFont {
            font_size: 24.0,
            ..Default::default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..Default::default()
        },
        SimulationStatusText,
    ));
}

pub fn update_simulation_ui(
    sim_time: Res<SimulationTime>,
    mut query: Query<(&mut Text, &mut TextColor), With<SimulationStatusText>>,
) {
    let Ok((mut text, mut color)) = query.single_mut() else {
        return;
    };

    let speed = sim_time.speed();
    let speed_str = format_speed(speed);

    if sim_time.paused() {
        **text = format!("PAUSED | Speed: {speed_str}");
        *color = TextColor(Color::srgb(1.0, 0.5, 0.5));
    } else {
        **text = format!("Speed: {speed_str}");
        *color = TextColor(Color::WHITE);
    }
}

fn format_speed(speed: f32) -> String {
    if speed >= 1.0 {
        format!("{}x", speed as i32)
    } else {
        let denom = (1.0 / speed).round() as i32;
        format!("1/{denom}x")
    }
}
