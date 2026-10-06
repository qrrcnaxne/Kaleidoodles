use std::f32::consts::TAU;

use bevy::{camera::RenderTarget, prelude::*, window::PrimaryWindow};

use crate::recording::{Recording, RecordingRenderTarget};

const PARTICLE_COUNT: usize = 1_600;
const PARTICLE_RADIUS: f32 = 2.2;
const PARTICLE_SPEED: f32 = 90.0;
const FLOW_CELL_SIZE: f32 = 360.0;

pub(super) struct FlowFieldPlugin;

impl Plugin for FlowFieldPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(Color::srgb(0.008, 0.012, 0.025)))
            .add_systems(PostStartup, setup)
            .add_systems(Update, move_particles);
    }
}

#[derive(Component)]
struct FlowParticle;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    recording: Option<Res<Recording>>,
    render_target: Option<Res<RecordingRenderTarget>>,
    window: Option<Single<&Window, With<PrimaryWindow>>>,
) {
    let viewport = match recording.as_ref() {
        Some(rec) => Vec2::new(rec.width as f32, rec.height as f32),
        None => {
            let window = window.as_ref().unwrap();
            Vec2::new(window.width(), window.height())
        }
    };
    if viewport.min_element() <= 0.0 {
        return;
    }

    if let Some(target) = render_target {
        commands.spawn((
            Camera2d,
            RenderTarget::Image(target.0.clone().into()),
            Transform::default(),
        ));
    } else {
        commands.spawn(Camera2d);
    }

    let particle_mesh = meshes.add(Circle::new(PARTICLE_RADIUS));
    let particle_material = materials.add(Color::srgb(0.42, 0.84, 1.0));

    for index in 0..PARTICLE_COUNT {
        let position = initial_position(index as u32 + 1, viewport);
        commands.spawn((
            Mesh2d(particle_mesh.clone()),
            MeshMaterial2d(particle_material.clone()),
            Transform::from_translation(position.extend(0.0)),
            FlowParticle,
        ));
    }
}

fn move_particles(
    time: Res<Time>,
    recording: Option<Res<Recording>>,
    window: Option<Single<&Window, With<PrimaryWindow>>>,
    mut particles: Query<&mut Transform, With<FlowParticle>>,
) {
    let viewport = match recording.as_ref() {
        Some(rec) => Vec2::new(rec.width as f32, rec.height as f32),
        None => {
            let window = window.as_ref().unwrap();
            Vec2::new(window.width(), window.height())
        }
    };
    let half_bounds = viewport * 0.5;
    if half_bounds.min_element() <= 0.0 {
        return;
    }

    let dt = match recording.as_ref() {
        Some(rec) => rec.fixed_dt,
        None => time.delta_secs(),
    };
    advance_particles(&mut particles, half_bounds, dt);
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

fn halton(mut index: u32, base: u32) -> f32 {
    let mut fraction = 1.0 / base as f32;
    let mut value = 0.0;

    while index > 0 {
        value += fraction * (index % base) as f32;
        index /= base;
        fraction /= base as f32;
    }

    value
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
    fn particles_wrap_across_each_viewport_edge() {
        let wrapped = wrap_position(Vec2::new(410.0, -410.0), Vec2::splat(400.0));
        assert_eq!(wrapped, Vec2::new(-390.0, 390.0));
    }
}
