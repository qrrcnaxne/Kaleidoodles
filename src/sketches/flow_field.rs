use std::f32::consts::TAU;

use bevy::{prelude::*, window::PrimaryWindow};

use super::common::{halton, simulation_delta, spawn_sketch_camera, viewport_size};
use crate::recording::{Recording, RecordingRenderTarget};

const PARTICLE_COUNT: usize = 1_600;
const PARTICLE_RADIUS: f32 = 2.2;
const PARTICLE_SPEED: f32 = 90.0;
const FLOW_CELL_SIZE: f32 = 360.0;
const COLOR_PALETTE_SIZE: usize = 24;
const COLOR_HUE_START: f32 = 120.0;
const COLOR_HUE_SPAN: f32 = 200.0;
const COLOR_DRIFT_SPEED: f32 = 8.0;
const COLOR_SATURATION: f32 = 0.9;
const COLOR_LIGHTNESS: f32 = 0.65;

pub(super) struct FlowFieldPlugin;

impl Plugin for FlowFieldPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(Color::srgb(0.008, 0.012, 0.025)))
            .add_systems(PostStartup, setup)
            .add_systems(Update, (move_particles, animate_particle_colors));
    }
}

#[derive(Component)]
struct FlowParticle;

#[derive(Resource)]
struct ParticlePalette {
    material_handles: Vec<Handle<ColorMaterial>>,
    elapsed_secs: f32,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    recording: Option<Res<Recording>>,
    render_target: Option<Res<RecordingRenderTarget>>,
    window: Option<Single<&Window, With<PrimaryWindow>>>,
) {
    let window_size = window
        .as_ref()
        .map(|window| Vec2::new(window.width(), window.height()));
    let Some(viewport) = viewport_size(recording.as_deref(), window_size) else {
        return;
    };
    spawn_sketch_camera(&mut commands, render_target.as_deref());

    let particle_mesh = meshes.add(Circle::new(PARTICLE_RADIUS));
    let palette_materials: Vec<_> = (0..COLOR_PALETTE_SIZE)
        .map(|index| {
            materials.add(Color::hsl(
                palette_hue(index, 0.0),
                COLOR_SATURATION,
                COLOR_LIGHTNESS,
            ))
        })
        .collect();

    for index in 0..PARTICLE_COUNT {
        let position = initial_position(index as u32 + 1, viewport);
        commands.spawn((
            Mesh2d(particle_mesh.clone()),
            MeshMaterial2d(palette_materials[index % COLOR_PALETTE_SIZE].clone()),
            Transform::from_translation(position.extend(0.0)),
            FlowParticle,
        ));
    }

    commands.insert_resource(ParticlePalette {
        material_handles: palette_materials,
        elapsed_secs: 0.0,
    });
}

fn move_particles(
    time: Res<Time>,
    recording: Option<Res<Recording>>,
    window: Option<Single<&Window, With<PrimaryWindow>>>,
    mut particles: Query<&mut Transform, With<FlowParticle>>,
) {
    let window_size = window
        .as_ref()
        .map(|window| Vec2::new(window.width(), window.height()));
    let Some(viewport) = viewport_size(recording.as_deref(), window_size) else {
        return;
    };
    let half_bounds = viewport * 0.5;

    let dt = simulation_delta(recording.as_deref(), time.delta_secs());
    advance_particles(&mut particles, half_bounds, dt);
}

fn animate_particle_colors(
    time: Res<Time>,
    recording: Option<Res<Recording>>,
    palette: Option<ResMut<ParticlePalette>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let Some(mut palette) = palette else {
        return;
    };

    let dt = simulation_delta(recording.as_deref(), time.delta_secs());
    palette.elapsed_secs += dt;
    let elapsed_secs = palette.elapsed_secs;

    for (index, handle) in palette.material_handles.iter().enumerate() {
        if let Some(mut material) = materials.get_mut(handle) {
            material.color = Color::hsl(
                palette_hue(index, elapsed_secs),
                COLOR_SATURATION,
                COLOR_LIGHTNESS,
            );
        }
    }
}

fn advance_particles(
    particles: &mut Query<&mut Transform, With<FlowParticle>>,
    half_bounds: Vec2,
    dt: f32,
) {
    let distance = PARTICLE_SPEED * dt;
    for mut transform in particles.iter_mut() {
        let position = transform.translation.truncate();
        let next_position = position + flow_direction(position) * distance;
        let wrapped_position = wrap_position(next_position, half_bounds);
        transform.translation.x = wrapped_position.x;
        transform.translation.y = wrapped_position.y;
    }
}

/// A smooth, periodic vortex field derived from a sinusoidal stream function.
fn flow_direction(position: Vec2) -> Vec2 {
    let frequency = TAU / FLOW_CELL_SIZE;
    let x = position.x * frequency;
    let y = position.y * frequency;
    let tangent = Vec2::new(x.sin() * y.cos(), -x.cos() * y.sin());

    tangent.normalize_or_zero()
}

/// Places particles in a repeatable low-discrepancy distribution without an RNG dependency.
fn initial_position(index: u32, viewport: Vec2) -> Vec2 {
    let x = halton(index, 2);
    let y = halton(index, 3);
    Vec2::new((x - 0.5) * viewport.x, (y - 0.5) * viewport.y)
}
fn palette_hue(index: usize, elapsed_secs: f32) -> f32 {
    let palette_offset = COLOR_HUE_SPAN * index as f32 / COLOR_PALETTE_SIZE as f32;
    (COLOR_HUE_START + palette_offset + elapsed_secs * COLOR_DRIFT_SPEED).rem_euclid(360.0)
}

fn wrap_position(position: Vec2, half_bounds: Vec2) -> Vec2 {
    Vec2::new(
        wrap_coordinate(position.x, half_bounds.x),
        wrap_coordinate(position.y, half_bounds.y),
    )
}

fn wrap_coordinate(value: f32, half_extent: f32) -> f32 {
    let span = half_extent * 2.0;
    (value + half_extent).rem_euclid(span) - half_extent
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flow_direction_has_unit_length_at_sample_points() {
        for position in [
            Vec2::new(23.0, 41.0),
            Vec2::new(125.0, -95.0),
            Vec2::new(-271.0, 173.0),
        ] {
            let length = flow_direction(position).length();
            assert!((length - 1.0).abs() < 1.0e-5);
        }
    }

    #[test]
    fn palette_colors_are_spread_out_and_drift_over_time() {
        let first_hue = palette_hue(0, 0.0);
        let later_palette_hue = palette_hue(COLOR_PALETTE_SIZE / 2, 0.0);
        let drifted_hue = palette_hue(0, 1.0);

        assert_eq!(first_hue, COLOR_HUE_START);
        assert!(later_palette_hue > first_hue);
        assert_eq!(drifted_hue, first_hue + COLOR_DRIFT_SPEED);
    }

    #[test]
    fn particles_wrap_across_each_viewport_edge() {
        let wrapped = wrap_position(Vec2::new(410.0, -410.0), Vec2::splat(400.0));
        assert_eq!(wrapped, Vec2::new(-390.0, 390.0));
    }
}
