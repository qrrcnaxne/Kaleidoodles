//! A deterministic hard-disk gas that nudges a much larger tracer.

use std::f32::consts::TAU;

use bevy::{prelude::*, window::PrimaryWindow};

use super::common::{simulation_delta, spawn_sketch_camera, viewport_size};
use crate::recording::{Recording, RecordingRenderTarget};

const MOLECULE_COUNT: usize = 220;
const TRACER_COUNT: usize = 5;
const MOLECULE_RADIUS: f32 = 3.0;
const MOLECULE_MASS: f32 = 1.0;
const MOLECULE_SPEED_MIN: f32 = 180.0;
const MOLECULE_SPEED_MAX: f32 = 250.0;
const TRACER_RADIUS: f32 = 14.0;
const TRACER_MASS: f32 = 16.0;
const HALF_BOUNDS: Vec2 = Vec2::new(270.0, 480.0);
const WALL_MARGIN: f32 = 18.0;
const FIXED_DT: f32 = 1.0 / 240.0;
const MAX_LIVE_DELTA: f32 = 0.1;
const MAX_TRAIL_POINTS: usize = 75;
const TRAIL_SAMPLE_DT: f32 = 0.04;
const TRACER_COLORS: [[f32; 3]; TRACER_COUNT] = [
    [0.97, 0.63, 0.31],
    [0.93, 0.39, 0.48],
    [0.75, 0.55, 0.96],
    [0.31, 0.76, 0.91],
    [0.55, 0.82, 0.42],
];
const TRACER_ANCHORS: [Vec2; TRACER_COUNT] = [
    Vec2::new(-150.0, -250.0),
    Vec2::new(150.0, -250.0),
    Vec2::ZERO,
    Vec2::new(-150.0, 250.0),
    Vec2::new(150.0, 250.0),
];

pub(super) struct BrownianMotionPlugin;

impl Plugin for BrownianMotionPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(Color::srgb(0.018, 0.031, 0.046)))
            .add_systems(PostStartup, setup)
            .add_systems(Update, (controls, advance_simulation, sync_visuals).chain());
    }
}

#[derive(Component)]
struct BodyVisual(usize);

#[derive(Component)]
struct MoleculeVisual;

#[derive(Component)]
struct TrailVisual {
    tracer_index: usize,
    point_index: usize,
}

#[derive(Component)]
struct WallVisual {
    position: Vec2,
    size: Vec2,
}

type BodyVisualQuery<'w, 's> = Query<
    'w,
    's,
    (&'static BodyVisual, &'static mut Transform),
    (Without<TrailVisual>, Without<WallVisual>),
>;
type TrailVisualQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static TrailVisual,
        &'static mut Transform,
        &'static mut Visibility,
    ),
    (Without<BodyVisual>, Without<WallVisual>),
>;
type WallVisualQuery<'w, 's> = Query<
    'w,
    's,
    (&'static WallVisual, &'static mut Transform),
    (Without<BodyVisual>, Without<TrailVisual>),
>;

#[derive(Resource)]
struct Gas {
    bodies: Vec<Body>,
    seed: u64,
    pending: f32,
    paused: bool,
    molecules_visible: bool,
    trails: Vec<Vec<Vec2>>,
    trail_pending: f32,
    scale: f32,
    just_reset: bool,
}

struct Body {
    position: Vec2,
    velocity: Vec2,
    radius: f32,
    mass: f32,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    recording: Option<Res<Recording>>,
    render_target: Option<Res<RecordingRenderTarget>>,
    mut window: Option<Single<&mut Window, With<PrimaryWindow>>>,
) {
    if let Some(window) = window.as_mut() {
        window.title = "Brownian motion".into();
        window.resolution.set(540.0, 960.0);
    }
    let window_size = window
        .as_ref()
        .map(|window| Vec2::new(window.width(), window.height()));
    let Some(viewport) = viewport_size(recording.as_deref(), window_size) else {
        return;
    };
    let scale = (viewport.x / (HALF_BOUNDS.x * 2.0)).min(viewport.y / (HALF_BOUNDS.y * 2.0));
    spawn_sketch_camera(&mut commands, render_target.as_deref());

    let seed = seed_from_args();
    let bodies = create_bodies(seed).expect("bounded body placement should fit the container");
    let molecule_mesh = meshes.add(Circle::new(MOLECULE_RADIUS));
    let tracer_mesh = meshes.add(Circle::new(TRACER_RADIUS));
    let molecule_material = materials.add(Color::srgb(0.36, 0.73, 0.77));
    let tracer_materials: Vec<_> = TRACER_COLORS
        .iter()
        .map(|[red, green, blue]| materials.add(Color::srgb(*red, *green, *blue)))
        .collect();

    // A thin, low-contrast outline keeps the portrait container legible.
    let wall_material = materials.add(Color::srgb(0.15, 0.23, 0.28));
    let wall_mesh = meshes.add(Rectangle::new(1.0, 1.0));
    let mut wall = |position: Vec2, size: Vec2| {
        commands.spawn((
            Mesh2d(wall_mesh.clone()),
            MeshMaterial2d(wall_material.clone()),
            Transform::from_translation((position * scale).extend(-1.0))
                .with_scale((size * scale).extend(1.0)),
            WallVisual { position, size },
        ));
    };
    let inner = HALF_BOUNDS - Vec2::splat(WALL_MARGIN);
    wall(Vec2::new(0.0, inner.y), Vec2::new(inner.x * 2.0, 2.0));
    wall(Vec2::new(0.0, -inner.y), Vec2::new(inner.x * 2.0, 2.0));
    wall(Vec2::new(inner.x, 0.0), Vec2::new(2.0, inner.y * 2.0));
    wall(Vec2::new(-inner.x, 0.0), Vec2::new(2.0, inner.y * 2.0));

    for (index, body) in bodies.iter().enumerate() {
        let tracer = index >= MOLECULE_COUNT;
        let tracer_index = index.saturating_sub(MOLECULE_COUNT);
        let entity = commands
            .spawn((
                Mesh2d(if tracer {
                    tracer_mesh.clone()
                } else {
                    molecule_mesh.clone()
                }),
                MeshMaterial2d(if tracer {
                    tracer_materials[tracer_index].clone()
                } else {
                    molecule_material.clone()
                }),
                Transform::from_translation((body.position * scale).extend(0.0))
                    .with_scale(Vec3::splat(scale)),
                BodyVisual(index),
            ))
            .id();
        if !tracer {
            commands.entity(entity).insert(MoleculeVisual);
        }
    }

    let trail_mesh = meshes.add(Circle::new(1.0));
    for (tracer_index, [red, green, blue]) in TRACER_COLORS.iter().enumerate() {
        for point_index in 0..MAX_TRAIL_POINTS {
            let age = (point_index + 1) as f32 / MAX_TRAIL_POINTS as f32;
            let material = materials.add(Color::srgba(*red, *green, *blue, 0.02 + age * 0.18));
            commands.spawn((
                Mesh2d(trail_mesh.clone()),
                MeshMaterial2d(material),
                Transform::default().with_scale(Vec3::splat(
                    scale * (1.0 + 1.1 * point_index as f32 / MAX_TRAIL_POINTS as f32),
                )),
                TrailVisual {
                    tracer_index,
                    point_index,
                },
                Visibility::Hidden,
            ));
        }
    }

    if let Some(mut window) = window {
        window.title = format!(
            "Brownian motion | seed {seed} | R reset  N new seed  M molecules  Space pause"
        );
    }

    commands.insert_resource(Gas {
        bodies,
        seed,
        pending: 0.0,
        paused: false,
        molecules_visible: true,
        trails: (0..TRACER_COUNT)
            .map(|_| Vec::with_capacity(MAX_TRAIL_POINTS))
            .collect(),
        trail_pending: 0.0,
        scale,
        just_reset: false,
    });
}

fn controls(
    keys: Res<ButtonInput<KeyCode>>,
    mut gas: ResMut<Gas>,
    mut molecules: Query<&mut Visibility, With<MoleculeVisual>>,
    mut window: Option<Single<&mut Window, With<PrimaryWindow>>>,
) {
    if keys.just_pressed(KeyCode::Space) {
        gas.paused = !gas.paused;
    }
    if keys.just_pressed(KeyCode::KeyM) {
        gas.molecules_visible = !gas.molecules_visible;
        let visibility = if gas.molecules_visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        for mut current in &mut molecules {
            *current = visibility;
        }
    }
    let reset = keys.just_pressed(KeyCode::KeyR);
    let new_seed = keys.just_pressed(KeyCode::KeyN);
    if reset || new_seed {
        let seed = if new_seed {
            gas.seed.wrapping_add(1)
        } else {
            gas.seed
        };
        if let Some(bodies) = create_bodies(seed) {
            gas.bodies = bodies;
            gas.seed = seed;
            gas.pending = 0.0;
            for trail in &mut gas.trails {
                trail.clear();
            }
            gas.trail_pending = 0.0;
            gas.paused = false;
            gas.just_reset = true;
            if let Some(window) = window.as_mut() {
                window.title = format!(
                    "Brownian motion | seed {} | R reset  N new seed  M molecules  Space pause",
                    gas.seed
                );
            }
        }
    }
}

fn advance_simulation(time: Res<Time>, recording: Option<Res<Recording>>, mut gas: ResMut<Gas>) {
    if gas.paused {
        return;
    }
    if gas.just_reset {
        gas.just_reset = false;
        return;
    }
    let dt = simulation_delta(recording.as_deref(), time.delta_secs());
    gas.pending += if recording.is_some() {
        dt
    } else {
        dt.min(MAX_LIVE_DELTA)
    };
    while gas.pending >= FIXED_DT {
        step(&mut gas.bodies);
        gas.pending -= FIXED_DT;
        gas.trail_pending += FIXED_DT;
        if gas.trail_pending >= TRAIL_SAMPLE_DT {
            let positions: Vec<_> = (0..TRACER_COUNT)
                .map(|index| gas.bodies[MOLECULE_COUNT + index].position)
                .collect();
            for (trail, position) in gas.trails.iter_mut().zip(positions) {
                trail.push(position);
                if trail.len() > MAX_TRAIL_POINTS {
                    trail.remove(0);
                }
            }
            gas.trail_pending -= TRAIL_SAMPLE_DT;
        }
    }
}

fn sync_visuals(
    recording: Option<Res<Recording>>,
    window: Option<Single<&Window, With<PrimaryWindow>>>,
    mut gas: ResMut<Gas>,
    mut bodies: BodyVisualQuery,
    mut trail: TrailVisualQuery,
    mut walls: WallVisualQuery,
) {
    let window_size = window
        .as_ref()
        .map(|window| Vec2::new(window.width(), window.height()));
    let Some(viewport) = viewport_size(recording.as_deref(), window_size) else {
        return;
    };
    gas.scale = (viewport.x / (HALF_BOUNDS.x * 2.0)).min(viewport.y / (HALF_BOUNDS.y * 2.0));
    for (visual, mut transform) in &mut bodies {
        transform.translation.x = gas.bodies[visual.0].position.x * gas.scale;
        transform.translation.y = gas.bodies[visual.0].position.y * gas.scale;
        transform.scale = Vec3::splat(gas.scale);
    }
    for (wall, mut transform) in &mut walls {
        transform.translation.x = wall.position.x * gas.scale;
        transform.translation.y = wall.position.y * gas.scale;
        transform.scale = (wall.size * gas.scale).extend(1.0);
    }
    for (visual, mut transform, mut visibility) in &mut trail {
        transform.scale = Vec3::splat(
            gas.scale * (1.0 + 1.1 * visual.point_index as f32 / MAX_TRAIL_POINTS as f32),
        );
        let points = &gas.trails[visual.tracer_index];
        let start = MAX_TRAIL_POINTS.saturating_sub(points.len());
        if visual.point_index < start {
            *visibility = Visibility::Hidden;
        } else {
            let position = points[visual.point_index - start];
            transform.translation = (position * gas.scale).extend(-0.2);
            *visibility = Visibility::Visible;
        }
    }
}

fn step(bodies: &mut [Body]) {
    for body in bodies.iter_mut() {
        body.position += body.velocity * FIXED_DT;
        let limit = HALF_BOUNDS - Vec2::splat(WALL_MARGIN + body.radius);
        for axis in 0..2 {
            if body.position[axis] > limit[axis] {
                body.position[axis] = limit[axis];
                body.velocity[axis] = -body.velocity[axis].abs();
            } else if body.position[axis] < -limit[axis] {
                body.position[axis] = -limit[axis];
                body.velocity[axis] = body.velocity[axis].abs();
            }
        }
    }
    for first in 0..bodies.len() {
        for second in first + 1..bodies.len() {
            let (left, right) = bodies.split_at_mut(second);
            resolve_pair(&mut left[first], &mut right[0], first, second);
        }
    }
}

fn resolve_pair(first: &mut Body, second: &mut Body, i: usize, j: usize) {
    let delta = second.position - first.position;
    let minimum = first.radius + second.radius;
    let distance_squared = delta.length_squared();
    if distance_squared >= minimum * minimum {
        return;
    }
    let normal = if distance_squared > 1.0e-8 {
        delta / distance_squared.sqrt()
    } else if (i + j).is_multiple_of(2) {
        Vec2::X
    } else {
        Vec2::Y
    };
    let inverse_first = 1.0 / first.mass;
    let inverse_second = 1.0 / second.mass;
    let inverse_sum = inverse_first + inverse_second;
    let overlap = minimum - distance_squared.sqrt();
    first.position -= normal * overlap * (inverse_first / inverse_sum);
    second.position += normal * overlap * (inverse_second / inverse_sum);

    let relative_speed = (second.velocity - first.velocity).dot(normal);
    if relative_speed < 0.0 {
        let impulse = -2.0 * relative_speed / inverse_sum;
        first.velocity -= normal * impulse * inverse_first;
        second.velocity += normal * impulse * inverse_second;
    }
}

fn create_bodies(seed: u64) -> Option<Vec<Body>> {
    let mut random = seed;
    let mut tracers = Vec::with_capacity(TRACER_COUNT);
    for anchor in TRACER_ANCHORS {
        let position = (0..1_000).find_map(|_| {
            let jitter = Vec2::new(
                (next_random(&mut random) * 2.0 - 1.0) * 18.0,
                (next_random(&mut random) * 2.0 - 1.0) * 18.0,
            );
            let candidate = anchor + jitter;
            let limit = HALF_BOUNDS - Vec2::splat(WALL_MARGIN + TRACER_RADIUS);
            let within_walls = candidate.abs().cmple(limit).all();
            let clears_tracers = tracers.iter().all(|tracer: &Body| {
                candidate.distance_squared(tracer.position) >= (2.0 * TRACER_RADIUS + 8.0).powi(2)
            });
            (within_walls && clears_tracers).then_some(candidate)
        })?;
        tracers.push(Body {
            position,
            velocity: Vec2::ZERO,
            radius: TRACER_RADIUS,
            mass: TRACER_MASS,
        });
    }
    let mut bodies = Vec::with_capacity(MOLECULE_COUNT + TRACER_COUNT);
    for _ in 0..MOLECULE_COUNT {
        let position = (0..20_000).find_map(|_| {
            let candidate = Vec2::new(
                (next_random(&mut random) * 2.0 - 1.0)
                    * (HALF_BOUNDS.x - WALL_MARGIN - MOLECULE_RADIUS),
                (next_random(&mut random) * 2.0 - 1.0)
                    * (HALF_BOUNDS.y - WALL_MARGIN - MOLECULE_RADIUS),
            );
            let clears_bodies = bodies.iter().all(|body: &Body| {
                candidate.distance_squared(body.position) >= (MOLECULE_RADIUS + body.radius).powi(2)
            });
            let clears_tracers = tracers.iter().all(|tracer| {
                candidate.distance_squared(tracer.position)
                    >= (MOLECULE_RADIUS + TRACER_RADIUS + 2.0).powi(2)
            });
            (clears_bodies && clears_tracers).then_some(candidate)
        })?;
        let angle = next_random(&mut random) * TAU;
        let speed = MOLECULE_SPEED_MIN
            + (MOLECULE_SPEED_MAX - MOLECULE_SPEED_MIN) * next_random(&mut random);
        bodies.push(Body {
            position,
            velocity: Vec2::from_angle(angle) * speed,
            radius: MOLECULE_RADIUS,
            mass: MOLECULE_MASS,
        });
    }
    bodies.extend(tracers);
    Some(bodies)
}

fn next_random(state: &mut u64) -> f32 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut value = *state;
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^= value >> 31;
    (value >> 40) as f32 / (1_u32 << 24) as f32
}

fn seed_from_args() -> u64 {
    let mut args = std::env::args().skip(1);
    while let Some(argument) = args.next() {
        if argument == "--seed"
            && let Some(seed) = args.next().and_then(|value| value.parse().ok())
        {
            return seed;
        }
    }
    42
}
