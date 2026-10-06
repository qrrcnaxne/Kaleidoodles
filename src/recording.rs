use std::path::PathBuf;

use bevy::prelude::*;

/// Configuration for deterministic frame-capture export.
///
/// Created from CLI args (`--record <dir>`) in `app::run` and consumed by the
/// active sketch to render a fixed-duration, fixed-framerate clip.
#[derive(Resource, Debug)]
pub struct Recording {
    pub output_dir: PathBuf,
    pub max_frames: u32,
    pub fixed_dt: f32,
    pub current_frame: u32,
}

impl Recording {
    /// Looks for `--record <directory>` in the process arguments.
    pub fn from_args() -> Option<Self> {
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            if arg == "--record" {
                let dir = args.next().unwrap_or_else(|| "frames".to_string());
                return Some(Self::new(dir, 60.0, 10.0));
            }
        }
        None
    }

    pub fn new(output_dir: impl Into<PathBuf>, frame_rate: f32, duration_seconds: f32) -> Self {
        let max_frames = (frame_rate * duration_seconds).max(0.0) as u32;
        Self {
            output_dir: output_dir.into(),
            max_frames,
            fixed_dt: 1.0 / frame_rate,
            current_frame: 0,
        }
    }
}
