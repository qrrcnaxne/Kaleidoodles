//! Individual creative-coding experiments.

use bevy::prelude::{App, Plugin};

mod attractors;
mod brownian_motion;
mod common;
mod crystal_growth;
mod dancer;
mod fish_tank;
mod flow_field;
mod reaction_diffusion;

/// The sketch the app currently runs; switch variants by changing this variant.
enum ActiveSketch {
    #[allow(dead_code)]
    BrownianMotion,
    CrystalGrowth,
    /// Kept selectable for the completed ecosystem experiment.
    #[allow(dead_code)]
    FishTank,
    /// Kept selectable for the earlier particles experiment.
    #[allow(dead_code)]
    FlowField,
    #[allow(dead_code)]
    Dancer,
    #[allow(dead_code)]
    Attractors,
    #[allow(dead_code)]
    ReactionDiffusion,
}

const ACTIVE_SKETCH: ActiveSketch = ActiveSketch::CrystalGrowth;

pub(crate) struct SketchesPlugin;

impl Plugin for SketchesPlugin {
    fn build(&self, app: &mut App) {
        match ACTIVE_SKETCH {
            ActiveSketch::BrownianMotion => app.add_plugins(brownian_motion::BrownianMotionPlugin),
            ActiveSketch::CrystalGrowth => app.add_plugins(crystal_growth::CrystalGrowthPlugin),
            ActiveSketch::ReactionDiffusion => {
                app.add_plugins(reaction_diffusion::ReactionDiffusionPlugin)
            }
            ActiveSketch::Attractors => app.add_plugins(attractors::AttractorsPlugin),
            ActiveSketch::FishTank => app.add_plugins(fish_tank::FishTankPlugin),
            ActiveSketch::FlowField => app.add_plugins(flow_field::FlowFieldPlugin),
            ActiveSketch::Dancer => app.add_plugins(dancer::DancerPlugin),
        };
    }
}

/// Runs the attractor numerical preflight before initializing Bevy rendering.
pub(crate) fn validate_attractors() -> bool {
    attractors::validate_candidates()
}
