use bevy::{
    camera::visibility::Visibility,
    color::Color,
    ecs::{
        query::With,
        system::{Commands, Query, Res},
    },
    text::{TextColor, TextFont},
    ui::{BackgroundColor, Node, PositionType, UiRect, Val, widget::Text},
};

use crate::{
    components::ui::{edge_info::EdgeInfoText, simulation::SimulationStatusText},
    resources::{
        selected_edge::SelectedEdge, simulation_time::SimulationTime, velocity::VelocitySync,
    },
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

pub fn setup_edge_info_ui(mut commands: Commands) {
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: 18.0,
            ..Default::default()
        },
        TextColor(Color::WHITE),
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            right: Val::Px(10.0),
            padding: UiRect::all(Val::Px(10.0)),
            ..Default::default()
        },
        Visibility::Hidden,
        EdgeInfoText,
    ));
}

pub fn update_edge_info_ui(
    selected_edge: Res<SelectedEdge>,
    velocity_sync: Res<VelocitySync>,
    mut query: Query<(&mut Text, &mut Visibility), With<EdgeInfoText>>,
) {
    let Ok((mut text, mut visibility)) = query.single_mut() else {
        return;
    };

    let Some(edge_idx) = selected_edge.0 else {
        *visibility = Visibility::Hidden;
        return;
    };

    *visibility = Visibility::Visible;

    let Ok(velocity) = velocity_sync.0.lock() else {
        **text = format!("Edge: {edge_idx}\n(velocity unavailable)");
        return;
    };

    if edge_idx >= velocity.len() {
        **text = format!("Edge: {edge_idx}\n(velocity unavailable)");
        return;
    }

    let [magnitude, angle] = velocity[edge_idx];
    **text = format!("Edge: {edge_idx}\nMagnitude: {magnitude:.4}\nAngle: {angle:.4} rad");
}
