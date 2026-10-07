//! Shared setup for 2D sketches.

use bevy::{camera::RenderTarget, prelude::*};

use crate::recording::{Recording, RecordingRenderTarget};

pub(super) fn simulation_delta(recording: Option<&Recording>, frame_delta: f32) -> f32 {
    recording.map_or(frame_delta, |recording| recording.fixed_dt)
}

/// Resolves the active sketch viewport from recording dimensions or the primary window.
pub(super) fn viewport_size(
    recording: Option<&Recording>,
    window_size: Option<Vec2>,
) -> Option<Vec2> {
    let viewport = recording
        .map(|recording| Vec2::new(recording.width as f32, recording.height as f32))
        .or(window_size)?;

    (viewport.min_element() > 0.0).then_some(viewport)
}

/// Spawns the sketch camera, targeting the recording image when recording is enabled.
pub(super) fn spawn_sketch_camera(
    commands: &mut Commands,
    render_target: Option<&RecordingRenderTarget>,
) {
    if let Some(target) = render_target {
        commands.spawn((
            Camera2d,
            RenderTarget::Image(target.0.clone().into()),
            Transform::default(),
        ));
    } else {
        commands.spawn(Camera2d);
    }
}

/// Halton low-discrepancy sequence value in (0, 1) for repeatable placements without an RNG.
pub(super) fn halton(mut index: u32, base: u32) -> f32 {
    let mut fraction = 1.0 / base as f32;
    let mut value = 0.0;

    while index > 0 {
        value += fraction * (index % base) as f32;
        index /= base;
        fraction /= base as f32;
    }

    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halton_values_stay_in_the_unit_interval() {
        assert_eq!(halton(1, 2), 0.5);
        assert_eq!(halton(1, 3), 1.0 / 3.0);
        let value = halton(17, 5);
        assert!((0.0..1.0).contains(&value));
    }

    #[test]
    fn recording_uses_a_fixed_simulation_timestep() {
        let recording = Recording::new("frames", 1080, 1920, 30.0, 20.0);
        assert_eq!(recording.fixed_dt, 1.0 / 30.0);
        assert_eq!(recording.max_frames, 600);
        assert_eq!(simulation_delta(Some(&recording), 0.25), recording.fixed_dt);
        assert_eq!(simulation_delta(None, 0.25), 0.25);
    }

    #[test]
    fn recording_dimensions_take_precedence_over_window_size() {
        let recording = Recording::new("frames", 1080, 1920, 60.0, 10.0);
        assert_eq!(
            viewport_size(Some(&recording), Some(Vec2::splat(640.0))),
            Some(Vec2::new(1080.0, 1920.0))
        );
    }

    #[test]
    fn window_dimensions_are_used_outside_recording() {
        let window_size = Vec2::new(800.0, 600.0);
        assert_eq!(viewport_size(None, Some(window_size)), Some(window_size));
    }

    #[test]
    fn missing_or_zero_sized_viewports_are_rejected() {
        assert_eq!(viewport_size(None, None), None);
        assert_eq!(viewport_size(None, Some(Vec2::new(0.0, 600.0))), None);
    }
}
