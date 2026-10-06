use bevy::prelude::*;
use bevy::window::{Window, WindowPlugin, WindowResolution};

use crate::{core::CorePlugin, recording::Recording, sketches::SketchesPlugin};

pub(crate) fn run() {
    let recording = Recording::from_args();

    let window_plugin = if let Some(ref rec) = recording {
        let _ = std::fs::create_dir_all(&rec.output_dir);
        WindowPlugin {
            primary_window: Some(Window {
                resolution: WindowResolution::new(1080, 1920),
                ..default()
            }),
            ..default()
        }
    } else {
        WindowPlugin::default()
    };

    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(window_plugin))
        .add_plugins(CorePlugin)
        .add_plugins(SketchesPlugin);

    if let Some(rec) = recording {
        app.insert_resource(rec);
    }

    app.run();
}
