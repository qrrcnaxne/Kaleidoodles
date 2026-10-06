//! Individual creative-coding experiments.

use bevy::prelude::{App, Plugin};

mod flow_field;

pub(crate) struct SketchesPlugin;

impl Plugin for SketchesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(flow_field::FlowFieldPlugin);
    }
}
