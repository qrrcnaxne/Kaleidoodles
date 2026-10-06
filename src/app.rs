use bevy::{
    prelude::*,
    window::{Window, WindowPlugin, WindowResolution},
};

use crate::{
    core::CorePlugin,
    recording::{Recording, RecordingPlugin},
    sketches::SketchesPlugin,
};

pub(crate) fn run() {
    let recording = Recording::from_args();
    let is_recording = recording.is_some();

    let default_plugins = if is_recording {
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: WindowResolution::new(100, 100),
                decorations: false,
                ..default()
            }),
            ..default()
        })
    } else {
        DefaultPlugins.set(WindowPlugin::default())
    };

    let mut app = App::new();
    app.add_plugins(default_plugins)
        .add_plugins(CorePlugin)
        .add_plugins(SketchesPlugin);

    if let Some(rec) = recording {
        let _ = std::fs::create_dir_all(&rec.output_dir);
        app.insert_resource(rec);
        app.add_plugins(RecordingPlugin);
    }

    app.run();
}
