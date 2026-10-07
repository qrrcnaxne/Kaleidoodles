use std::time::Duration;

use bevy::{
    app::ScheduleRunnerPlugin,
    prelude::*,
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};

use crate::{
    core::CorePlugin,
    recording::{HeadlessSimulationPlugin, Recording, RecordingPlugin},
    sketches::SketchesPlugin,
};

pub(crate) fn run() {
    let run_config = Recording::from_args();
    let is_headless = run_config.is_some();

    let default_plugins = if is_headless {
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .disable::<WinitPlugin>()
    } else {
        DefaultPlugins.set(WindowPlugin::default())
    };

    let mut app = App::new();
    app.add_plugins(default_plugins)
        .add_plugins(CorePlugin)
        .add_plugins(SketchesPlugin);

    if let Some(run) = run_config {
        let captures_frames = run.captures_frames();
        let frame_duration = Duration::from_secs_f64(f64::from(run.fixed_dt));
        if captures_frames {
            let _ = std::fs::create_dir_all(&run.output_dir);
        }
        app.insert_resource(run);
        if captures_frames {
            app.add_plugins(RecordingPlugin);
        } else {
            app.add_plugins(HeadlessSimulationPlugin);
        }
        // Frame capture paces to the recording rate; simulation-only runs go
        // as fast as the machine allows so parameter sweeps finish quickly.
        app.add_plugins(ScheduleRunnerPlugin::run_loop(if captures_frames {
            frame_duration
        } else {
            Duration::ZERO
        }));
    }

    app.run();
}
