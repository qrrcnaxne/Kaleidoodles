//! Seeded, lattice-based diffusion-limited aggregation replayed over one minute.

use bevy::{prelude::*, window::PrimaryWindow};

use super::common::{simulation_delta, spawn_sketch_camera, viewport_size};
use crate::recording::{Recording, RecordingRenderTarget};

const GRID_WIDTH: usize = 180;
const GRID_HEIGHT: usize = 320;
const WALKER_COUNT: usize = 128;
const WALK_STEPS_PER_TICK: usize = 16;
const REPLAY_SECONDS: f32 = 60.0;
// Leave a short completed tail to absorb render readback and entity-update latency.
const CAPTURE_TAIL_FRAMES: u32 = 8;
const MAX_GENERATION_TICKS: usize = 100_000;
const MAX_LIVE_DELTA: f32 = 0.1;
const CELL_SIZE: f32 = 3.0;
const DEFAULT_SEED: u64 = 42;
const SEED_Y: usize = GRID_HEIGHT - 4;
const TARGET_BOTTOM: usize = 17;
const DIRECTIONS: [(isize, isize); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

pub(super) struct CrystalGrowthPlugin;

impl Plugin for CrystalGrowthPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(Color::srgb(0.012, 0.022, 0.04)))
            .add_systems(PostStartup, setup)
            .add_systems(Update, (controls, advance_growth, sync_layout).chain());
    }
}

#[derive(Component)]
struct WalkerVisual(usize);

#[derive(Component)]
struct AggregateVisual {
    x: usize,
    y: usize,
}

#[derive(Resource)]
struct Growth {
    sequence: Option<GrowthSequence>,
    failure: Option<String>,
    walker_entities: Vec<Entity>,
    seed: u64,
    elapsed: f32,
    event_cursor: usize,
    scale: f32,
    paused: bool,
    complete: bool,
    skip_frame: bool,
    cell_mesh: Handle<Mesh>,
    cell_material: Handle<ColorMaterial>,
    seed_material: Handle<ColorMaterial>,
}

struct GrowthSequence {
    /// Each entry packs x and y into one u32 and records the generation tick.
    placements: Vec<(u32, u32)>,
    /// One packed position per walker, for every generation tick including tick zero.
    walker_snapshots: Vec<u32>,
    ticks: usize,
}

#[derive(Clone, Copy)]
struct Walker {
    x: usize,
    y: usize,
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
        window.title = "Crystal growth".into();
        window.resolution.set(540.0, 960.0);
    }
    let window_size = window
        .as_ref()
        .map(|window| Vec2::new(window.width(), window.height()));
    let Some(viewport) = viewport_size(recording.as_deref(), window_size) else {
        return;
    };
    let scale = (viewport.x / (GRID_WIDTH as f32 * CELL_SIZE))
        .min(viewport.y / (GRID_HEIGHT as f32 * CELL_SIZE));
    spawn_sketch_camera(&mut commands, render_target.as_deref());

    let cell_mesh = meshes.add(Rectangle::new(CELL_SIZE, CELL_SIZE));
    let walker_mesh = meshes.add(Circle::new(CELL_SIZE * 0.34));
    let cell_material = materials.add(Color::srgb(0.37, 0.76, 0.91));
    let seed_material = materials.add(Color::srgb(0.98, 0.72, 0.39));
    let walker_material = materials.add(Color::srgb(0.83, 0.91, 0.96));
    let (sequence, failure) = make_sequence(DEFAULT_SEED);

    commands.spawn((
        Mesh2d(cell_mesh.clone()),
        MeshMaterial2d(seed_material.clone()),
        Transform::from_translation(cell_position(GRID_WIDTH / 2, SEED_Y, scale))
            .with_scale(Vec3::splat(scale)),
        AggregateVisual {
            x: GRID_WIDTH / 2,
            y: SEED_Y,
        },
    ));

    let initial_walkers = sequence
        .as_ref()
        .map(|sequence| &sequence.walker_snapshots[..WALKER_COUNT]);
    let mut walker_entities = Vec::with_capacity(WALKER_COUNT);
    for index in 0..WALKER_COUNT {
        let walker = initial_walkers
            .map(|walkers| unpack_walker(walkers[index]))
            .unwrap_or(Walker {
                x: GRID_WIDTH / 2,
                y: SEED_Y - 12,
            });
        let entity = commands
            .spawn((
                Mesh2d(walker_mesh.clone()),
                MeshMaterial2d(walker_material.clone()),
                Transform::from_translation(cell_position(walker.x, walker.y, scale))
                    .with_scale(Vec3::splat(scale)),
                WalkerVisual(index),
            ))
            .id();
        walker_entities.push(entity);
    }

    let growth = Growth {
        sequence,
        failure,
        walker_entities,
        seed: DEFAULT_SEED,
        elapsed: 0.0,
        event_cursor: 0,
        scale,
        paused: false,
        complete: false,
        skip_frame: false,
        cell_mesh,
        cell_material,
        seed_material,
    };
    update_window_title(&mut window, &growth);
    commands.insert_resource(growth);
}

fn controls(
    keys: Res<ButtonInput<KeyCode>>,
    mut growth: ResMut<Growth>,
    mut commands: Commands,
    aggregate_entities: Query<Entity, With<AggregateVisual>>,
    mut walker_visuals: Query<&mut Transform, With<WalkerVisual>>,
    mut window: Option<Single<&mut Window, With<PrimaryWindow>>>,
) {
    if keys.just_pressed(KeyCode::Space) {
        growth.paused = !growth.paused;
    }
    let new_seed = keys.just_pressed(KeyCode::KeyN);
    let reset = keys.just_pressed(KeyCode::KeyR);
    if !new_seed && !reset {
        return;
    }

    let seed = if new_seed {
        growth.seed.wrapping_add(1)
    } else {
        growth.seed
    };
    let (sequence, failure) = make_sequence(seed);
    for entity in &aggregate_entities {
        commands.entity(entity).despawn();
    }
    commands.spawn((
        Mesh2d(growth.cell_mesh.clone()),
        MeshMaterial2d(growth.seed_material.clone()),
        Transform::from_translation(cell_position(GRID_WIDTH / 2, SEED_Y, growth.scale))
            .with_scale(Vec3::splat(growth.scale)),
        AggregateVisual {
            x: GRID_WIDTH / 2,
            y: SEED_Y,
        },
    ));

    growth.sequence = sequence;
    growth.failure = failure;
    growth.seed = seed;
    growth.elapsed = 0.0;
    growth.event_cursor = 0;
    growth.paused = false;
    growth.complete = false;
    growth.skip_frame = true;
    if let Some(sequence) = &growth.sequence {
        for (index, entity) in growth.walker_entities.iter().enumerate() {
            if let Ok(mut transform) = walker_visuals.get_mut(*entity) {
                let walker = unpack_walker(sequence.walker_snapshots[index]);
                transform.translation = cell_position(walker.x, walker.y, growth.scale);
                transform.scale = Vec3::splat(growth.scale);
            }
        }
    }
    update_window_title(&mut window, &growth);
}

fn advance_growth(
    time: Res<Time>,
    recording: Option<Res<Recording>>,
    mut growth: ResMut<Growth>,
    mut walkers_visuals: Query<(&WalkerVisual, &mut Transform)>,
    mut commands: Commands,
    mut window: Option<Single<&mut Window, With<PrimaryWindow>>>,
) {
    if growth.paused || growth.complete || growth.sequence.is_none() {
        return;
    }
    if growth.skip_frame {
        growth.skip_frame = false;
        return;
    }

    let progress = if let Some(recording) = recording.as_deref() {
        captured_progress(recording)
    } else {
        let dt = simulation_delta(None, time.delta_secs()).min(MAX_LIVE_DELTA);
        growth.elapsed = (growth.elapsed + dt).min(REPLAY_SECONDS);
        growth.elapsed / REPLAY_SECONDS
    };

    let total_ticks = growth
        .sequence
        .as_ref()
        .expect("sequence checked above")
        .ticks;
    let target_tick = progress * total_ticks as f32;
    loop {
        let placement = growth
            .sequence
            .as_ref()
            .and_then(|sequence| sequence.placements.get(growth.event_cursor).copied());
        let Some((tick, packed)) = placement else {
            break;
        };
        if tick as f32 > target_tick {
            break;
        }
        let (x, y) = unpack_position(packed);
        commands.spawn((
            Mesh2d(growth.cell_mesh.clone()),
            MeshMaterial2d(growth.cell_material.clone()),
            Transform::from_translation(cell_position(x, y, growth.scale))
                .with_scale(Vec3::splat(growth.scale)),
            AggregateVisual { x, y },
        ));
        growth.event_cursor += 1;
    }

    let sequence = growth.sequence.as_ref().expect("sequence checked above");
    let snapshot_tick = (target_tick.floor() as usize).min(sequence.ticks);
    let snapshot_start = snapshot_tick * WALKER_COUNT;
    for (visual, mut transform) in &mut walkers_visuals {
        let walker = unpack_walker(sequence.walker_snapshots[snapshot_start + visual.0]);
        transform.translation = cell_position(walker.x, walker.y, growth.scale);
        transform.scale = Vec3::splat(growth.scale);
    }

    if progress >= 1.0 {
        growth.complete = true;
        update_window_title(&mut window, &growth);
    }
}

fn sync_layout(
    recording: Option<Res<Recording>>,
    window: Option<Single<&Window, With<PrimaryWindow>>>,
    mut growth: ResMut<Growth>,
    mut aggregates: Query<(&AggregateVisual, &mut Transform), Without<WalkerVisual>>,
    mut walkers: Query<(&WalkerVisual, &mut Transform), Without<AggregateVisual>>,
) {
    let window_size = window
        .as_ref()
        .map(|window| Vec2::new(window.width(), window.height()));
    let Some(viewport) = viewport_size(recording.as_deref(), window_size) else {
        return;
    };
    let scale = (viewport.x / (GRID_WIDTH as f32 * CELL_SIZE))
        .min(viewport.y / (GRID_HEIGHT as f32 * CELL_SIZE));
    if (scale - growth.scale).abs() < f32::EPSILON {
        return;
    }
    growth.scale = scale;
    for (aggregate, mut transform) in &mut aggregates {
        transform.translation = cell_position(aggregate.x, aggregate.y, scale);
        transform.scale = Vec3::splat(scale);
    }
    let Some(sequence) = growth.sequence.as_ref() else {
        return;
    };
    let progress = recording.as_deref().map_or_else(
        || (growth.elapsed / REPLAY_SECONDS).min(1.0),
        captured_progress,
    );
    let snapshot_tick = (progress * sequence.ticks as f32).floor() as usize;
    let snapshot_start = snapshot_tick.min(sequence.ticks) * WALKER_COUNT;
    for (visual, mut transform) in &mut walkers {
        let walker = unpack_walker(sequence.walker_snapshots[snapshot_start + visual.0]);
        transform.translation = cell_position(walker.x, walker.y, scale);
        transform.scale = Vec3::splat(scale);
    }
}

fn make_sequence(seed: u64) -> (Option<GrowthSequence>, Option<String>) {
    match generate_sequence(seed) {
        Ok(sequence) => (Some(sequence), None),
        Err(ticks) => (
            None,
            Some(format!(
                "seed {seed} did not reach the bottom within {ticks} generation ticks"
            )),
        ),
    }
}

fn generate_sequence(seed: u64) -> Result<GrowthSequence, usize> {
    let mut occupied = vec![false; GRID_WIDTH * GRID_HEIGHT];
    let seed_x = GRID_WIDTH / 2;
    occupied[cell_index(seed_x, SEED_Y)] = true;
    let mut min_x = seed_x;
    let mut max_x = seed_x;
    let mut bottom = SEED_Y;
    let mut rng = seed.max(1);
    let mut walkers = Vec::with_capacity(WALKER_COUNT);
    for _ in 0..WALKER_COUNT {
        walkers.push(launch_walker(&mut rng, min_x, max_x, bottom));
    }

    let mut placements = Vec::new();
    let mut walker_snapshots = Vec::with_capacity(2048 * WALKER_COUNT);
    walker_snapshots.extend(walkers.iter().copied().map(pack_walker));

    for tick in 1..=MAX_GENERATION_TICKS {
        let mut complete = false;
        for walker in &mut walkers {
            for _ in 0..WALK_STEPS_PER_TICK {
                let random_index = (next_random(&mut rng) >> 32) as usize;
                let (dx, dy) = DIRECTIONS[random_index % DIRECTIONS.len()];
                let next_x = walker.x as isize + dx;
                let next_y = walker.y as isize + dy;
                if next_x <= 0
                    || next_x >= GRID_WIDTH as isize - 1
                    || next_y <= 0
                    || next_y >= GRID_HEIGHT as isize - 1
                {
                    *walker = launch_walker(&mut rng, min_x, max_x, bottom);
                    continue;
                }
                walker.x = next_x as usize;
                walker.y = next_y as usize;

                if occupied[cell_index(walker.x, walker.y)] {
                    *walker = launch_walker(&mut rng, min_x, max_x, bottom);
                    continue;
                }
                if touches_cluster(&occupied, walker.x, walker.y) {
                    occupied[cell_index(walker.x, walker.y)] = true;
                    min_x = min_x.min(walker.x);
                    max_x = max_x.max(walker.x);
                    bottom = bottom.min(walker.y);
                    placements.push((tick as u32, pack_position(walker.x, walker.y)));
                    *walker = launch_walker(&mut rng, min_x, max_x, bottom);
                    if bottom <= TARGET_BOTTOM {
                        complete = true;
                        break;
                    }
                }
            }
            if complete {
                break;
            }
        }
        walker_snapshots.extend(walkers.iter().copied().map(pack_walker));
        if complete {
            return Ok(GrowthSequence {
                placements,
                walker_snapshots,
                ticks: tick,
            });
        }
    }
    Err(MAX_GENERATION_TICKS)
}

fn captured_progress(recording: &Recording) -> f32 {
    if recording.max_frames == 0 {
        return 1.0;
    }
    let playback_frames = recording
        .max_frames
        .saturating_sub(CAPTURE_TAIL_FRAMES)
        .max(1);
    (recording.completed_frames as f32 / playback_frames as f32).min(1.0)
}

fn launch_walker(rng: &mut u64, min_x: usize, max_x: usize, bottom: usize) -> Walker {
    let spread = 22usize;
    let low = min_x.saturating_sub(spread).max(1);
    let high = (max_x + spread).min(GRID_WIDTH - 2);
    let random_offset = (next_random(rng) >> 32) as usize;
    let x = low + random_offset % (high - low + 1);
    let y = bottom.saturating_sub(12).max(2);
    Walker { x, y }
}

fn update_window_title(
    window: &mut Option<Single<&mut Window, With<PrimaryWindow>>>,
    growth: &Growth,
) {
    if let Some(window) = window.as_mut() {
        window.title = if let Some(failure) = &growth.failure {
            format!("Crystal growth | generation limit reached | {failure}")
        } else if growth.complete {
            format!(
                "Crystal growth | 60 seconds complete | seed {}",
                growth.seed
            )
        } else {
            format!(
                "Crystal growth | seed {} | R reset  N new seed  Space pause",
                growth.seed
            )
        };
    }
}

fn pack_walker(walker: Walker) -> u32 {
    pack_position(walker.x, walker.y)
}

fn unpack_walker(position: u32) -> Walker {
    let (x, y) = unpack_position(position);
    Walker { x, y }
}

fn pack_position(x: usize, y: usize) -> u32 {
    x as u32 | ((y as u32) << 8)
}

fn unpack_position(position: u32) -> (usize, usize) {
    ((position & 0xff) as usize, (position >> 8) as usize)
}

fn touches_cluster(occupied: &[bool], x: usize, y: usize) -> bool {
    DIRECTIONS.iter().any(|(dx, dy)| {
        let neighbor_x = (x as isize + dx) as usize;
        let neighbor_y = (y as isize + dy) as usize;
        occupied[cell_index(neighbor_x, neighbor_y)]
    })
}

fn cell_index(x: usize, y: usize) -> usize {
    y * GRID_WIDTH + x
}

fn cell_position(x: usize, y: usize, scale: f32) -> Vec3 {
    let centered_x = (x as f32 + 0.5 - GRID_WIDTH as f32 * 0.5) * CELL_SIZE;
    let centered_y = (y as f32 + 0.5 - GRID_HEIGHT as f32 * 0.5) * CELL_SIZE;
    Vec3::new(centered_x * scale, centered_y * scale, 0.0)
}

fn next_random(state: &mut u64) -> u64 {
    let mut value = *state;
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    *state = value;
    value
}
