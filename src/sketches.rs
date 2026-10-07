//! Individual creative-coding experiments.

use bevy::prelude::{App, Plugin};

mod common;
mod dancer;
mod fish_tank;
mod flow_field;

/// The sketch the app currently runs; switch variants by changing this variant.
enum ActiveSketch {
    /// Kept selectable for the completed ecosystem experiment.
    #[allow(dead_code)]
    FishTank,
    /// Kept selectable for the earlier particles experiment.
    #[allow(dead_code)]
    FlowField,
    Dancer,
}

const ACTIVE_SKETCH: ActiveSketch = ActiveSketch::Dancer;

pub(crate) struct SketchesPlugin;

impl Plugin for SketchesPlugin {
    fn build(&self, app: &mut App) {
        match ACTIVE_SKETCH {
            ActiveSketch::FishTank => app.add_plugins(fish_tank::FishTankPlugin),
            ActiveSketch::FlowField => app.add_plugins(flow_field::FlowFieldPlugin),
            ActiveSketch::Dancer => app.add_plugins(dancer::DancerPlugin),
        };
    }
}
