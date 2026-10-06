//! App-wide systems and resources shared across sketches.

use bevy::prelude::{App, Plugin};

pub(crate) struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, _app: &mut App) {
        // Add shared infrastructure here when an experiment calls for it.
    }
}
