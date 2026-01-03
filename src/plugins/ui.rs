use bevy::app::{App, Plugin, Startup, Update};

use crate::systems::ui::{setup_simulation_ui, update_simulation_ui};

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_simulation_ui);
        app.add_systems(Update, update_simulation_ui);
    }
}
