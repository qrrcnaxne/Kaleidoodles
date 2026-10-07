//! A 2D fish tank ecosystem: four trophic levels of CPU boids, sinking food,
//! energy-based reproduction (meals bank energy; two meal-units fund each
//! birth, half of the investment transfers to the newborn), and starvation.
//! Optional immigration insurance stocks adults into any species below a
//! population floor so oscillations cannot reach extinction. The default
//! parameters are a calibrated equilibrium found by a randomized parameter
//! search (see `scripts/sweep.sh`); every knob can be overridden from the CLI.
//! A `--demo` profile trades the equilibrium for legibility instead: a few
//! large, slow fish, with ring markers on births and deaths.

use std::collections::HashSet;
use std::f32::consts::TAU;

use bevy::{
    asset::RenderAssetUsages, ecs::schedule::IntoScheduleConfigs, ecs::system::SystemParam,
    prelude::*, render::mesh::PrimitiveTopology, sprite_render::AlphaMode2d, window::PrimaryWindow,
};

use super::common::{halton, simulation_delta, spawn_sketch_camera, viewport_size};
use crate::recording::{Recording, RecordingRenderTarget};

/// Species are trophic levels: species 0 eats pellets, species n eats species
/// n-1, and nothing eats anything two or more levels below it.
const SPECIES_COUNTS: [usize; 4] = [25, 8, 3, 2];

/// Hard cap so a runaway population cannot tank the frame rate; feeding still
/// removes prey past the cap but skips the split.
const MAX_FISH: usize = 1_200;

/// Energy value of one pellet; only level-1 fish can eat pellets.
const PELLET_ENERGY: f32 = 1.0;
/// Embodied energy invested in a fish; half transfers to its newborn.
const PREY_BODY_ENERGY: f32 = 2.0;
const PREY_TRANSFER: f32 = PREY_BODY_ENERGY * 0.5;
/// Newborns inherit a one-unit reserve; initial founders start with two so
/// each level has time to establish before its first reproduction.
const INITIAL_ENERGY: f32 = BIRTH_COST;
const NEWBORN_ENERGY: f32 = PREY_TRANSFER;
/// When a species is below the immigration floor, one stocked adult swims in
/// per such species this often (simulated seconds); floor 0 disables it.
const IMMIGRATION_INTERVAL: f32 = 5.0;
/// A fish must bank two meal-units above its starting reserve before it births.
const BIRTH_COST: f32 = PREY_BODY_ENERGY;
const BIRTH_THRESHOLD: f32 = NEWBORN_ENERGY + BIRTH_COST;

const DART_MIN_DELAY: f32 = 3.0;
const DART_MAX_DELAY: f32 = 5.0;
const DART_PROBABILITY: f32 = 0.8;
const DART_FULL_SECS: f32 = 1.0;
const DART_TAPER_SECS: f32 = 2.0;
const DART_TOTAL_SECS: f32 = DART_FULL_SECS + DART_TAPER_SECS;
const DART_ACCEL: f32 = 2.0;
const DART_DRAG: f32 = 6.0;

const FLEE_WEIGHT: f32 = 6.0;
const HUNT_WEIGHT: f32 = 2.5;
/// Fear level up to which a fish still hunts (times EcoParams.fear_gate_scale).
const FEAR_HUNT_LIMIT: f32 = 0.5;

const PELLET_RADIUS: f32 = 4.0;
const PELLET_SINK_SPEED: f32 = 40.0;
const PELLET_SEEK_RADIUS: f32 = 500.0;
const PELLET_EAT_RADIUS: f32 = 16.0;
const PELLETS_PER_EVENT: usize = 1;

/// Tuning knobs for equilibrium search runs, parsed from CLI flags so many
/// headless simulations can sweep parameter space in parallel. Scales
/// multiply the baseline species table; the seed offsets the deterministic
/// layout and hash streams so each run samples a fresh trajectory.
#[derive(Resource, Debug)]
struct EcoParams {
    seed: u32,
    /// Average pellets spawned per simulated second.
    pellet_rate: f32,
    drain_scale: f32,
    catch_scale: f32,
    lifespan_scale: f32,
    /// Multiplier on the flee steering weight; smaller means prey escape less surely.
    flee_scale: f32,
    /// Multiplier on the hunt steering weight; larger means predators pursue harder.
    hunt_scale: f32,
    /// Multiplier on the predator-prey contact distance for fish-on-fish meals.
    reach_scale: f32,
    /// Multiplier on the fear level up to which a fish still hunts; 0 restores
    /// hunt-only-when-completely-calm behaviour.
    fear_gate_scale: f32,
    /// Multiplier on predator levels' (1..3) speed, giving them a chase advantage.
    pursuit_scale: f32,
    /// Multiplier on every species' hunger cooldown; smaller means more meal
    /// attempts per second.
    hunger_scale: f32,
    /// Multiplier on the founder population of each trophic level.
    founders_scale: f32,
    /// Explicit founder counts per trophic level, used instead of the baseline
    /// counts when set. The demo needs a balanced pyramid rather than the
    /// equilibrium's heavily scaled one.
    founder_counts: Option<[usize; 4]>,
    /// Multiplier on every body and distance measurement: fish length, flocking
    /// and wall distances, hunt and fear radii, pellet size, and pellet reach.
    /// Strike distance is derived from body lengths, so it scales with them.
    size_scale: f32,
    /// Multiplier on every species' swimming speed and on pellet sink speed.
    speed_scale: f32,
    /// Ring markers where a fish is born and where one dies, so the population
    /// rules are visible rather than merely happening.
    markers: bool,
    /// Species with fewer fish than this receive a stocked adult every
    /// [`IMMIGRATION_INTERVAL`] seconds; 0 disables immigration insurance.
    immigration_floor: u32,
}

impl Default for EcoParams {
    /// Calibrated equilibrium for a 180 s reel: founders 13.5x the baseline
    /// counts, plentiful food, long lifespans, brisk but modest predators, and
    /// an immigration floor of 2 as a no-extinction backstop. Over these 180 s
    /// every level stays well clear of the floor (cyan 200+, lime 60+, orange
    /// 15+, magenta 21+); past roughly five minutes the magenta and orange
    /// levels thin out and lean on immigration, so a longer reel needs a
    /// different trade-off.
    fn default() -> Self {
        Self {
            seed: 0,
            pellet_rate: 15.663,
            drain_scale: 0.66,
            catch_scale: 0.31,
            lifespan_scale: 1.94,
            flee_scale: 0.39,
            hunt_scale: 1.91,
            reach_scale: 5.44,
            fear_gate_scale: 1.0,
            pursuit_scale: 1.84,
            hunger_scale: 0.49,
            founders_scale: 13.51,
            founder_counts: None,
            immigration_floor: 2,
            size_scale: 1.0,
            speed_scale: 1.0,
            markers: false,
        }
    }
}

impl EcoParams {
    fn from_args() -> Self {
        let mut params = Self::default();
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--seed" => {
                    if let Some(seed) = args.next().and_then(|v| v.parse().ok()) {
                        params.seed = seed;
                    }
                }
                "--pellet-rate" => {
                    if let Some(rate) = args
                        .next()
                        .and_then(|v| v.parse().ok())
                        .filter(|v: &f32| *v > 0.0)
                    {
                        params.pellet_rate = rate;
                    }
                }
                "--drain-scale" => {
                    if let Some(scale) = args
                        .next()
                        .and_then(|v| v.parse().ok())
                        .filter(|v: &f32| *v > 0.0)
                    {
                        params.drain_scale = scale;
                    }
                }
                "--catch-scale" => {
                    if let Some(scale) = args
                        .next()
                        .and_then(|v| v.parse().ok())
                        .filter(|v: &f32| *v > 0.0)
                    {
                        params.catch_scale = scale;
                    }
                }
                "--lifespan-scale" => {
                    if let Some(scale) = args
                        .next()
                        .and_then(|v| v.parse().ok())
                        .filter(|v: &f32| *v > 0.0)
                    {
                        params.lifespan_scale = scale;
                    }
                }
                "--flee-scale" => {
                    if let Some(scale) = args
                        .next()
                        .and_then(|v| v.parse().ok())
                        .filter(|v: &f32| *v > 0.0)
                    {
                        params.flee_scale = scale;
                    }
                }
                "--hunt-scale" => {
                    if let Some(scale) = args
                        .next()
                        .and_then(|v| v.parse().ok())
                        .filter(|v: &f32| *v > 0.0)
                    {
                        params.hunt_scale = scale;
                    }
                }
                "--reach-scale" => {
                    if let Some(scale) = args
                        .next()
                        .and_then(|v| v.parse().ok())
                        .filter(|v: &f32| *v > 0.0)
                    {
                        params.reach_scale = scale;
                    }
                }
                "--fear-gate-scale" => {
                    if let Some(scale) = args.next().and_then(|v| v.parse::<f32>().ok()) {
                        params.fear_gate_scale = scale.max(0.0);
                    }
                }
                "--pursuit-scale" => {
                    if let Some(scale) = args
                        .next()
                        .and_then(|v| v.parse().ok())
                        .filter(|v: &f32| *v > 0.0)
                    {
                        params.pursuit_scale = scale;
                    }
                }
                "--hunger-scale" => {
                    if let Some(scale) = args
                        .next()
                        .and_then(|v| v.parse().ok())
                        .filter(|v: &f32| *v > 0.0)
                    {
                        params.hunger_scale = scale;
                    }
                }
                "--founders-scale" => {
                    if let Some(scale) = args
                        .next()
                        .and_then(|v| v.parse().ok())
                        .filter(|v: &f32| *v > 0.0)
                    {
                        params.founders_scale = scale;
                    }
                }
                "--counts" => {
                    if let Some(counts) = args.next().and_then(|v| parse_counts(&v)) {
                        params.founder_counts = Some(counts);
                    }
                }
                "--immigration-floor" => {
                    if let Some(floor) = args.next().and_then(|v| v.parse().ok()) {
                        params.immigration_floor = floor;
                    }
                }
                "--size-scale" => {
                    if let Some(scale) = args
                        .next()
                        .and_then(|v| v.parse().ok())
                        .filter(|v: &f32| *v > 0.0)
                    {
                        params.size_scale = scale;
                    }
                }
                "--speed-scale" => {
                    if let Some(scale) = args
                        .next()
                        .and_then(|v| v.parse().ok())
                        .filter(|v: &f32| *v > 0.0)
                    {
                        params.speed_scale = scale;
                    }
                }
                "--markers" => params.markers = true,
                "--demo" => params = Self::demo(),
                _ => {}
            }
        }
        params
    }

    /// Legibility profile for the rules demo: a few large, slow fish in a tank
    /// sparse enough to watch a single fish eat another and split. It replaces
    /// the equilibrium tuning wholesale, because thinning the population and
    /// slowing it down changes the food web; the numbers here are chosen so the
    /// rules keep firing often enough to be seen over a short clip.
    fn demo() -> Self {
        Self {
            seed: 0,
            pellet_rate: 2.6,
            drain_scale: 1.0,
            catch_scale: 6.0,
            lifespan_scale: 3.5,
            flee_scale: 0.1,
            hunt_scale: 2.0,
            reach_scale: 1.0,
            fear_gate_scale: 1.0,
            pursuit_scale: 2.0,
            hunger_scale: 0.2,
            founders_scale: 1.0,
            founder_counts: Some([46, 22, 18, 6]),
            immigration_floor: 2,
            size_scale: 3.0,
            speed_scale: 0.25,
            markers: true,
        }
    }
}

/// Parses founder counts written as `a/b/c/d`, one per trophic level.
fn parse_counts(value: &str) -> Option<[usize; 4]> {
    let parts: Vec<usize> = value
        .split('/')
        .map(|part| part.parse().ok())
        .collect::<Option<Vec<usize>>>()?;
    let counts: [usize; 4] = parts.try_into().ok()?;
    (counts.iter().all(|c| *c > 0)).then_some(counts)
}

pub(super) struct FishTankPlugin;

impl Plugin for FishTankPlugin {
    fn build(&self, app: &mut App) {
        let params = EcoParams::from_args();
        let table = SpeciesTable::schools_with(&params);
        let seed = params.seed;
        app.insert_resource(ClearColor(Color::srgb(0.015, 0.05, 0.1)))
            .insert_resource(params)
            .insert_resource(table)
            .insert_resource(FishCounter {
                next: 10_000 + 100_000 * seed,
            })
            .add_systems(PostStartup, setup)
            .init_resource::<EcoStats>()
            .add_systems(
                Update,
                (
                    update_fish,
                    lifecycle,
                    immigrate,
                    feed,
                    spawn_event_markers,
                    spawn_pellets,
                    sink_pellets,
                    animate_event_pulses,
                    update_stats,
                    log_population,
                )
                    .chain(),
            )
            .init_resource::<TankEvents>();
    }
}

#[derive(Component)]
struct Fish {
    species: usize,
    velocity: Vec2,
    id: u32,
    darting: bool,
    dart_elapsed: f32,
    dart_timer: f32,
    dart_count: u32,
    /// Seconds until the fish may eat again; it hunts only at zero.
    hunger: f32,
    /// Energy reserve; meals add energy, reproduction spends it, and starvation drains it.
    energy: f32,
    /// Number of predator contact attempts, used to make catch rolls deterministic.
    hunt_attempts: u32,
    age: f32,
    lifespan: f32,
}

/// Sinking food that only the smallest fish steer toward and consume.
#[derive(Component)]
struct Pellet;

/// Per-run event tallies for equilibrium diagnostics: how many fish each
/// level bore, caught, and had stocked by immigration insurance, and how
/// deaths split between age and starvation.
#[derive(Resource, Default)]
struct EcoStats {
    births: [u32; 4],
    catches: [u32; 4],
    attempts: [u32; 4],
    stocked: [u32; 4],
    aged_out: u32,
    starved: u32,
}

/// Corner readout of population counts for tuning the ecosystem equilibrium.
#[derive(Component)]
struct StatsText;

/// Mesh and material handles shared by every fish and pellet.
#[derive(Resource, Clone)]
struct TankAssets {
    species: Vec<(Handle<Mesh>, Handle<ColorMaterial>)>,
    pellet_mesh: Handle<Mesh>,
    pellet_material: Handle<ColorMaterial>,
    /// Unit ring scaled per event marker.
    ring_mesh: Handle<Mesh>,
}

/// Kinds of population event worth marking on screen.
#[derive(Clone, Copy, PartialEq, Eq)]
enum TankEventKind {
    /// A fish appeared: born from a parent or stocked by immigration insurance.
    Born,
    /// A predator ate another fish.
    Eaten,
    /// A fish died of old age or starvation.
    Died,
}

struct TankEvent {
    kind: TankEventKind,
    position: Vec2,
    /// Already scaled with the species, so markers keep pace with fish size.
    radius: f32,
}

/// Events raised during a frame and consumed by [`spawn_event_markers`].
#[derive(Resource, Default)]
struct TankEvents(Vec<TankEvent>);

/// Ring that expands and fades over [`EventPulse::duration`] seconds.
#[derive(Component)]
struct EventPulse {
    age: f32,
    duration: f32,
    radius: f32,
    base_alpha: f32,
    color: Color,
    material: Handle<ColorMaterial>,
}

/// Spawns pellets on a rate-driven jittered cadence at random positions in the tank.
#[derive(Resource)]
struct PelletTimer {
    countdown: f32,
    spawned: u32,
    seed: u32,
}

/// Deterministic source of fresh fish ids for newborn fish.
#[derive(Resource)]
struct FishCounter {
    next: u32,
}

/// Resolves the sketch viewport and simulation timestep from recording state
/// or the primary window, so every system reads them the same way.
#[derive(SystemParam)]
struct SketchViewport<'w, 's> {
    recording: Option<Res<'w, Recording>>,
    window: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
}

impl SketchViewport<'_, '_> {
    fn resolve(&self) -> Option<Vec2> {
        let window_size = self
            .window
            .single()
            .ok()
            .map(|window| Vec2::new(window.width(), window.height()));
        viewport_size(self.recording.as_deref(), window_size)
    }

    fn delta(&self, time: &Time) -> f32 {
        simulation_delta(self.recording.as_deref(), time.delta_secs())
    }
}

struct SpeciesConfig {
    color: Color,
    body_length: f32,
    max_speed: f32,
    max_force: f32,
    neighbor_radius: f32,
    /// Radius inside which fish are pushed away from every other fish, regardless of species.
    personal_space: f32,
    repulsion_weight: f32,
    alignment_weight: f32,
    cohesion_weight: f32,
    wall_avoid_weight: f32,
    wall_margin: f32,
    /// Seconds a fish lives before dying of old age.
    lifespan: f32,
    /// Seconds between meal attempts; a fish hunts only when this has elapsed.
    hunger_secs: f32,
    /// Energy lost per second when the fish is not feeding.
    energy_drain: f32,
    /// Probability a contact attempt catches prey; unused by pellet-feeders.
    catch_chance: f32,
    /// Distance at which a hungry fish starts chasing its single-level-lower prey.
    hunt_radius: f32,
    /// Distance at which a fish flees its single-level-higher predator.
    fear_radius: f32,
    darts: bool,
}

#[derive(Resource, Default)]
struct SpeciesTable(Vec<SpeciesConfig>);

impl SpeciesTable {
    /// Baseline species table with the given equilibrium-search scales applied.
    fn schools_with(params: &EcoParams) -> Self {
        let mut table = Self::schools();
        for (level, config) in &mut table.0.iter_mut().enumerate() {
            // Geometry first, so every derived distance stays proportional to
            // the bodies it describes.
            config.body_length *= params.size_scale;
            config.neighbor_radius *= params.size_scale;
            config.personal_space *= params.size_scale;
            config.wall_margin *= params.size_scale;
            config.hunt_radius *= params.size_scale;
            config.fear_radius *= params.size_scale;
            // Forces scale with speed so acceleration stays proportional and
            // the motion keeps its character at any speed.
            config.max_speed *= params.speed_scale;
            config.max_force *= params.speed_scale;
            config.energy_drain *= params.drain_scale;
            config.catch_chance = (config.catch_chance * params.catch_scale).min(1.0);
            config.lifespan *= params.lifespan_scale;
            if level > 0 {
                config.max_speed *= params.pursuit_scale;
                config.max_force *= params.pursuit_scale;
            }
            config.hunger_secs *= params.hunger_scale;
        }
        table
    }

    fn schools() -> Self {
        Self(vec![
            // Level 1 — cyan schoolers: graze on sinking pellets.
            SpeciesConfig {
                color: Color::hsl(190.0, 0.85, 0.68),
                body_length: 5.0,
                max_speed: 140.0,
                max_force: 140.0,
                neighbor_radius: 60.0,
                personal_space: 12.0,
                repulsion_weight: 4.0,
                alignment_weight: 1.0,
                cohesion_weight: 1.0,
                wall_avoid_weight: 2.0,
                wall_margin: 40.0,
                lifespan: 45.0,
                hunger_secs: 2.0,
                energy_drain: 0.01,
                catch_chance: 1.0,
                hunt_radius: PELLET_SEEK_RADIUS,
                fear_radius: 200.0,
                darts: true,
            },
            // Level 2 — lime cruisers: hunt cyan.
            SpeciesConfig {
                color: Color::hsl(100.0, 0.9, 0.62),
                body_length: 9.0,
                max_speed: 150.0,
                max_force: 160.0,
                neighbor_radius: 90.0,
                personal_space: 26.0,
                repulsion_weight: 6.0,
                alignment_weight: 0.6,
                cohesion_weight: 1.4,
                wall_avoid_weight: 2.0,
                wall_margin: 60.0,
                lifespan: 90.0,
                hunger_secs: 3.0,
                energy_drain: 0.003,
                catch_chance: 0.13,
                hunt_radius: 400.0,
                fear_radius: 220.0,
                darts: true,
            },
            // Level 3 — orange hunters: hunt lime.
            SpeciesConfig {
                color: Color::hsl(18.0, 0.95, 0.58),
                body_length: 14.0,
                max_speed: 160.0,
                max_force: 170.0,
                neighbor_radius: 120.0,
                personal_space: 40.0,
                repulsion_weight: 5.0,
                alignment_weight: 0.2,
                cohesion_weight: 0.4,
                wall_avoid_weight: 1.5,
                wall_margin: 80.0,
                lifespan: 150.0,
                hunger_secs: 4.0,
                energy_drain: 0.0015,
                catch_chance: 0.1,
                hunt_radius: 450.0,
                fear_radius: 240.0,
                darts: true,
            },
            // Level 4 — magenta apex: hunt orange; hunted by nothing.
            SpeciesConfig {
                color: Color::hsl(305.0, 0.9, 0.68),
                body_length: 26.0,
                max_speed: 170.0,
                max_force: 180.0,
                neighbor_radius: 100.0,
                personal_space: 30.0,
                repulsion_weight: 4.0,
                alignment_weight: 0.2,
                cohesion_weight: 0.4,
                wall_avoid_weight: 3.0,
                wall_margin: 50.0,
                lifespan: 240.0,
                hunger_secs: 5.0,
                energy_drain: 0.001,
                catch_chance: 0.05,
                hunt_radius: 500.0,
                fear_radius: 0.0,
                darts: false,
            },
        ])
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    species: Res<SpeciesTable>,
    params: Res<EcoParams>,
    sketch_view: SketchViewport,
    render_target: Option<Res<RecordingRenderTarget>>,
) {
    spawn_sketch_camera(&mut commands, render_target.as_deref());

    let Some(viewport) = sketch_view.resolve() else {
        return;
    };

    let assets: Vec<_> = species
        .0
        .iter()
        .map(|config| {
            (
                meshes.add(fish_mesh(config.body_length)),
                materials.add(config.color),
            )
        })
        .collect();
    commands.insert_resource(TankAssets {
        species: assets.clone(),
        pellet_mesh: meshes.add(Circle::new(PELLET_RADIUS * params.size_scale)),
        // Pale, low-saturation specks: saturated means fish, pale means food,
        // and no pellet hue collides with a species colour.
        pellet_material: materials.add(Color::hsl(48.0, 0.35, 0.92)),
        ring_mesh: meshes.add(Annulus::new(0.72, 1.0)),
    });

    commands.insert_resource(PelletTimer {
        countdown: 1.0 / params.pellet_rate,
        spawned: 0,
        seed: params.seed,
    });

    let mut index = 1 + 4_000 * params.seed;
    let founder_counts = params.founder_counts.unwrap_or(SPECIES_COUNTS);
    for (species_index, count) in founder_counts.iter().enumerate() {
        let count = ((*count as f32) * params.founders_scale).round().max(1.0) as usize;
        let config = &species.0[species_index];
        for _ in 0..count {
            let position = initial_position(index, viewport);
            let angle = halton(index, 5) * TAU;
            let velocity = Vec2::from_angle(angle) * config.max_speed;
            let (mesh, material) = &assets[species_index];
            commands.spawn((
                Mesh2d(mesh.clone()),
                MeshMaterial2d(material.clone()),
                Transform::from_translation(position.extend(0.0)),
                Fish {
                    species: species_index,
                    velocity,
                    id: index,
                    darting: false,
                    dart_elapsed: 0.0,
                    dart_timer: halton(index, 7) * DART_MAX_DELAY,
                    dart_count: 0,
                    hunger: config.hunger_secs * halton(index, 11),
                    energy: INITIAL_ENERGY,
                    hunt_attempts: 0,
                    age: config.lifespan * halton(index, 13),
                    lifespan: config.lifespan,
                },
            ));
            index += 1;
        }
    }

    // Population readout for live tuning; kept out of recorded clips.
    if sketch_view.recording.is_none() {
        commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(8.0),
                left: Val::Px(8.0),
                ..default()
            },
            Text::new(""),
            TextFont {
                font_size: FontSize::Px(15.0),
                ..default()
            },
            TextColor(Color::srgb(0.8, 0.9, 1.0)),
            StatsText,
        ));
    }
}

#[derive(SystemParam)]
struct FeedContext<'w> {
    counter: ResMut<'w, FishCounter>,
    stats: ResMut<'w, EcoStats>,
    params: Res<'w, EcoParams>,
    events: ResMut<'w, TankEvents>,
}

#[derive(SystemParam)]
struct ImmigrationCtx<'w> {
    params: Res<'w, EcoParams>,
    species: Res<'w, SpeciesTable>,
    assets: Res<'w, TankAssets>,
    counter: ResMut<'w, FishCounter>,
    stats: ResMut<'w, EcoStats>,
    events: ResMut<'w, TankEvents>,
}

type FishQuery<'w, 's> =
    Query<'w, 's, (Entity, &'static mut Transform, &'static mut Fish), Without<Pellet>>;

///Steers every fish: flocking by default, fleeing its one-level-higher
/// predator when close, and hunting pellets (level 1) or one-level-lower prey
/// when hungry.
fn update_fish(
    time: Res<Time>,
    viewport: SketchViewport,
    species: Res<SpeciesTable>,
    params: Res<EcoParams>,
    pellets: Query<&Transform, With<Pellet>>,
    mut fish: FishQuery,
) {
    let Some(view) = viewport.resolve() else {
        return;
    };
    let half_bounds = view * 0.5;
    let dt = viewport.delta(&time);

    let snapshot: Vec<(Vec2, Vec2, usize)> = fish
        .iter()
        .map(|(_, transform, fish)| {
            (
                transform.translation.truncate(),
                fish.velocity,
                fish.species,
            )
        })
        .collect();
    let mut updates = Vec::with_capacity(snapshot.len());

    for (entity, transform, fish) in fish.iter() {
        let position = transform.translation.truncate();
        let config = &species.0[fish.species];
        let radius_sq = config.neighbor_radius * config.neighbor_radius;

        let (darting, dart_elapsed, dart_timer, dart_count, dart_factor) = if config.darts {
            advance_dart(
                fish.id,
                fish.darting,
                fish.dart_elapsed,
                fish.dart_timer,
                fish.dart_count,
                dt,
            )
        } else {
            (
                fish.darting,
                fish.dart_elapsed,
                fish.dart_timer,
                fish.dart_count,
                0.0,
            )
        };

        let mut repulsion = Vec2::ZERO;
        let mut alignment = Vec2::ZERO;
        let mut cohesion = Vec2::ZERO;
        let mut neighbors = 0usize;
        for (other_position, other_velocity, other_species) in &snapshot {
            let offset = *other_position - position;
            let distance_sq = offset.length_squared();
            if distance_sq <= 0.0 {
                continue;
            }
            let distance = distance_sq.sqrt();
            // Predators tolerate their own prey's personal space; without this,
            // mutual repulsion keeps every fish farther apart than the strike distance.
            let other_is_prey = fish.species > 0 && *other_species == fish.species - 1;
            if distance < config.personal_space && !other_is_prey {
                let strength = repulsion_strength(distance, config.personal_space);
                repulsion -= offset / distance * strength;
            }
            if *other_species == fish.species && distance_sq < radius_sq {
                alignment += *other_velocity;
                cohesion += *other_position;
                neighbors += 1;
            }
        }

        // Flee the nearest fish that is exactly one level above this one.
        let mut fear = 0.0;
        let mut threat_position = None;
        if fish.species + 1 < species.0.len() {
            let predator_species = fish.species + 1;
            let mut nearest = config.fear_radius;
            for (other_position, _, other_species) in &snapshot {
                if *other_species != predator_species {
                    continue;
                }
                let distance = (*other_position - position).length();
                if distance < nearest {
                    nearest = distance;
                    threat_position = Some(*other_position);
                }
            }
            if threat_position.is_some() {
                fear = fear_factor(nearest, config.fear_radius);
            }
        }

        // A hungry fish hunts instead of schooling.
        let mut hunt_target = None;
        if fear < FEAR_HUNT_LIMIT * params.fear_gate_scale && fish.hunger <= 0.0 {
            if fish.species == 0 {
                let mut nearest = PELLET_SEEK_RADIUS;
                for pellet_transform in pellets.iter() {
                    let distance = (pellet_transform.translation.truncate() - position).length();
                    if distance < nearest {
                        nearest = distance;
                        hunt_target = Some(pellet_transform.translation.truncate());
                    }
                }
            } else {
                let prey_species = fish.species - 1;
                let mut nearest = config.hunt_radius;
                for (other_position, _, other) in &snapshot {
                    if *other != prey_species {
                        continue;
                    }
                    let distance = (*other_position - position).length();
                    if distance < nearest {
                        nearest = distance;
                        hunt_target = Some(*other_position);
                    }
                }
            }
        }

        let mut acceleration =
            wall_avoidance(position, half_bounds, config.wall_margin) * config.wall_avoid_weight;
        acceleration += repulsion * config.max_speed * config.repulsion_weight;
        if fear > 0.0 {
            if let Some(threat) = threat_position {
                let flee_desired =
                    (position - threat).normalize_or_zero() * config.max_speed - fish.velocity;
                acceleration += flee_desired * FLEE_WEIGHT * fear * params.flee_scale;
            }
        } else if let Some(target) = hunt_target {
            let hunt_desired =
                (target - position).normalize_or_zero() * config.max_speed - fish.velocity;
            acceleration += hunt_desired * HUNT_WEIGHT * params.hunt_scale;
        } else if neighbors > 0 {
            let count = neighbors as f32;
            let alignment_desired =
                (alignment / count).normalize_or_zero() * config.max_speed - fish.velocity;
            let cohesion_desired = ((cohesion / count) - position).normalize_or_zero()
                * config.max_speed
                - fish.velocity;
            acceleration += alignment_desired * config.alignment_weight
                + cohesion_desired * config.cohesion_weight;
        }

        acceleration = acceleration.clamp_length_max(config.max_force);
        if dart_factor > 0.0 {
            acceleration +=
                fish.velocity.normalize_or_zero() * config.max_speed * DART_ACCEL * dart_factor;
        }
        let mut velocity = fish.velocity + acceleration * dt;
        let speed = velocity.length();
        if speed > config.max_speed {
            let target = if dart_factor > 0.0 {
                speed - (speed - config.max_speed) * (DART_DRAG * dt).min(1.0)
            } else {
                config.max_speed
            };
            velocity *= target / speed;
        }
        let new_position = position + velocity * dt;
        let (new_position, velocity) = bounce_off_walls(new_position, velocity, half_bounds);
        updates.push((
            entity,
            new_position,
            velocity,
            darting,
            dart_elapsed,
            dart_timer,
            dart_count,
        ));
    }

    for (
        (_, mut transform, mut fish),
        (_update_entity, position, velocity, darting, dart_elapsed, dart_timer, dart_count),
    ) in fish.iter_mut().zip(updates)
    {
        transform.translation = position.extend(0.0);
        if velocity.length_squared() > 0.0001 {
            transform.rotation = Quat::from_rotation_z(velocity.to_angle());
        }
        fish.velocity = velocity;
        fish.darting = darting;
        fish.dart_elapsed = dart_elapsed;
        fish.dart_timer = dart_timer;
        fish.dart_count = dart_count;
    }
}

/// Ages and starves fish, despawning those that run out of energy or lifespan.
fn lifecycle(
    time: Res<Time>,
    viewport: SketchViewport,
    species: Res<SpeciesTable>,
    mut fish: FishQuery,
    mut commands: Commands,
    mut stats: ResMut<EcoStats>,
    mut events: ResMut<TankEvents>,
) {
    let dt = viewport.delta(&time);

    let mut expired = Vec::new();
    for (entity, transform, mut fish) in fish.iter_mut() {
        fish.hunger -= dt;
        fish.age += dt;
        fish.energy -= species.0[fish.species].energy_drain * dt;
        if fish.age >= fish.lifespan || fish.energy <= 0.0 {
            if fish.age >= fish.lifespan {
                stats.aged_out += 1;
            } else {
                stats.starved += 1;
            }
            events.0.push(TankEvent {
                kind: TankEventKind::Died,
                position: transform.translation.truncate(),
                radius: species.0[fish.species].body_length * 1.2,
            });
            expired.push(entity);
        }
    }
    for entity in expired {
        commands.entity(entity).despawn();
    }
}

/// Insurance against extinction: every [`IMMIGRATION_INTERVAL`] seconds, each
/// species below the immigration floor gets one fertile adult stocked in at a
/// deterministic random spot. Floor 0 keeps the tank a closed system.
fn immigrate(
    time: Res<Time>,
    viewport: SketchViewport,
    mut ctx: ImmigrationCtx,
    mut commands: Commands,
    fish: Query<&Fish>,
    mut timer: Local<f32>,
) {
    if ctx.params.immigration_floor == 0 {
        return;
    }
    let Some(view) = viewport.resolve() else {
        return;
    };
    let dt = viewport.delta(&time);
    *timer -= dt;
    if *timer > 0.0 {
        return;
    }
    *timer = IMMIGRATION_INTERVAL;

    let mut counts = [0usize; 4];
    for fish in fish.iter() {
        counts[fish.species] += 1;
    }
    for (level, config) in ctx.species.0.iter().enumerate() {
        if counts[level] >= ctx.params.immigration_floor as usize {
            continue;
        }
        let id = ctx.counter.next;
        ctx.counter.next += 1;
        ctx.stats.stocked[level] += 1;
        let position = Vec2::new(
            (hash01(stream_seed(id, 0, 31)) * 2.0 - 1.0) * view.x * 0.45,
            (hash01(stream_seed(id, 1, 31)) * 2.0 - 1.0) * view.y * 0.45,
        );
        ctx.events.0.push(TankEvent {
            kind: TankEventKind::Born,
            position,
            radius: config.body_length * 1.6,
        });
        let heading = (hash01(stream_seed(id, 2, 31)) * 2.0 - 1.0) * std::f32::consts::PI;
        let (mesh, material) = &ctx.assets.species[level];
        commands.spawn((
            Mesh2d(mesh.clone()),
            MeshMaterial2d(material.clone()),
            Transform::from_translation(position.extend(0.0)),
            Fish {
                species: level,
                velocity: Vec2::from_angle(heading) * config.max_speed * 0.5,
                id,
                darting: false,
                dart_elapsed: 0.0,
                dart_timer: DART_MIN_DELAY
                    + (DART_MAX_DELAY - DART_MIN_DELAY) * hash01(stream_seed(id, 3, 31)),
                dart_count: 0,
                hunger: 0.0,
                energy: INITIAL_ENERGY,
                hunt_attempts: 0,
                age: 0.0,
                lifespan: config.lifespan,
            },
        ));
    }
}

/// Hungry fish attempt to eat: level 1 takes a pellet, level n hunts the
/// nearest one-level-lower fish. Predator contact succeeds by species-specific
/// chance; meals bank energy and reproduction transfers half the birth cost
/// into one newborn while the parent keeps its age and life.
fn feed(
    viewport: SketchViewport,
    species: Res<SpeciesTable>,
    assets: Res<TankAssets>,
    pellets: Query<(Entity, &Transform), With<Pellet>>,
    mut ctx: FeedContext,
    mut commands: Commands,
    mut fish: FishQuery,
) {
    if viewport.resolve().is_none() {
        return;
    }

    let snapshot: Vec<(Entity, Vec2, usize)> = fish
        .iter()
        .map(|(entity, transform, fish)| (entity, transform.translation.truncate(), fish.species))
        .collect();

    let mut consumed: HashSet<Entity> = HashSet::new();
    let mut births: Vec<(usize, Vec2, f32)> = Vec::new();

    for (entity, transform, mut fish) in fish.iter_mut() {
        if fish.hunger > 0.0 || consumed.contains(&entity) {
            continue;
        }
        let position = transform.translation.truncate();
        let config = &species.0[fish.species];

        if fish.species == 0 {
            let mut nearest = PELLET_EAT_RADIUS * ctx.params.size_scale;
            let mut target = None;
            for (pellet_entity, pellet_transform) in pellets.iter() {
                if consumed.contains(&pellet_entity) {
                    continue;
                }
                let distance = (pellet_transform.translation.truncate() - position).length();
                if distance < nearest {
                    nearest = distance;
                    target = Some(pellet_entity);
                }
            }
            if let Some(pellet) = target {
                consumed.insert(pellet);
                fish.energy += PELLET_ENERGY;
                fish.hunger = config.hunger_secs;
                ctx.stats.catches[0] += 1;
                if fish.energy >= BIRTH_THRESHOLD {
                    fish.energy -= BIRTH_COST;
                    births.push((0, position, fish.velocity.to_angle()));
                }
            }
        } else {
            let prey_species = fish.species - 1;
            let mut nearest = f32::MAX;
            let mut target: Option<(Entity, Vec2, f32)> = None;
            for (prey_entity, prey_position, prey) in &snapshot {
                if *prey_entity == entity || consumed.contains(prey_entity) {
                    continue;
                }
                if *prey != prey_species {
                    continue;
                }
                let prey_config = &species.0[*prey];
                let distance = (*prey_position - position).length();
                if distance < nearest
                    && distance
                        < eat_distance(config.body_length, prey_config.body_length)
                            * ctx.params.reach_scale
                {
                    nearest = distance;
                    target = Some((*prey_entity, *prey_position, prey_config.body_length));
                }
            }
            if let Some((prey, prey_position, prey_body)) = target {
                fish.hunger = config.hunger_secs;
                ctx.stats.attempts[fish.species] += 1;
                let caught =
                    hash01(stream_seed(fish.id, fish.hunt_attempts, 9)) < config.catch_chance;
                fish.hunt_attempts = fish.hunt_attempts.wrapping_add(1);
                if caught {
                    consumed.insert(prey);
                    ctx.stats.catches[fish.species] += 1;
                    ctx.events.0.push(TankEvent {
                        kind: TankEventKind::Eaten,
                        position: prey_position,
                        radius: prey_body * 2.2,
                    });
                    fish.energy += PREY_TRANSFER;
                    if fish.energy >= BIRTH_THRESHOLD {
                        fish.energy -= BIRTH_COST;
                        births.push((fish.species, position, fish.velocity.to_angle()));
                    }
                }
            }
        }
    }

    let mut population = fish.iter().len();
    for (species_index, position, heading) in births {
        if population < MAX_FISH {
            ctx.events.0.push(TankEvent {
                kind: TankEventKind::Born,
                position,
                radius: species.0[species_index].body_length * 1.6,
            });
            reproduce(
                &mut commands,
                &mut ctx.counter,
                species_index,
                position,
                heading,
                &species,
                &assets,
            );
            ctx.stats.births[species_index] += 1;
            population += 1;
        }
    }
    for entity in consumed {
        commands.entity(entity).despawn();
    }
}

/// Contact distance at which a predator eats its prey.
fn eat_distance(predator_length: f32, prey_length: f32) -> f32 {
    (predator_length + prey_length) * 0.5
}

/// Spawns one newborn fish of the same species at the eater's position; the
/// eater itself keeps its current age and life.
fn reproduce(
    commands: &mut Commands,
    counter: &mut FishCounter,
    species_index: usize,
    position: Vec2,
    heading: f32,
    species: &SpeciesTable,
    assets: &TankAssets,
) {
    let config = &species.0[species_index];
    let (mesh, material) = &assets.species[species_index];
    let id = counter.next;
    counter.next += 1;
    let side = if hash01(stream_seed(id, 0, 6)) < 0.5 {
        -1.0
    } else {
        1.0
    };
    let angle = heading + side * (0.3 + 0.4 * hash01(stream_seed(id, 0, 5)));
    commands.spawn((
        Mesh2d(mesh.clone()),
        MeshMaterial2d(material.clone()),
        Transform::from_translation(position.extend(0.0)),
        Fish {
            species: species_index,
            velocity: Vec2::from_angle(angle) * config.max_speed * 0.5,
            id,
            darting: false,
            dart_elapsed: 0.0,
            dart_timer: DART_MIN_DELAY
                + (DART_MAX_DELAY - DART_MIN_DELAY) * hash01(stream_seed(id, 0, 7)),
            dart_count: 0,
            hunger: config.hunger_secs * (0.5 + 0.5 * hash01(stream_seed(id, 0, 8))),
            energy: NEWBORN_ENERGY,
            hunt_attempts: 0,
            age: 0.0,
            lifespan: config.lifespan,
        },
    ));
}

/// Spawns pellets on a rate-driven jittered cadence at random positions in the tank.
fn spawn_pellets(
    time: Res<Time>,
    viewport: SketchViewport,
    params: Res<EcoParams>,
    mut timer: ResMut<PelletTimer>,
    assets: Res<TankAssets>,
    mut commands: Commands,
) {
    let Some(view) = viewport.resolve() else {
        return;
    };
    let dt = viewport.delta(&time);

    timer.countdown -= dt;
    if timer.countdown > 0.0 {
        return;
    }
    timer.spawned += 1;
    let salt = 999 + timer.seed * 17;
    for i in 0..PELLETS_PER_EVENT {
        let x_offset =
            (hash01(stream_seed(salt, timer.spawned * 10 + i as u32, 3)) - 0.5) * view.x * 0.8;
        let y_offset =
            (hash01(stream_seed(salt, timer.spawned * 10 + i as u32, 5)) - 0.5) * view.y * 0.8;
        commands.spawn((
            Mesh2d(assets.pellet_mesh.clone()),
            MeshMaterial2d(assets.pellet_material.clone()),
            Transform::from_translation(Vec3::new(x_offset, y_offset, 0.0)),
            Pellet,
        ));
    }
    let interval = 1.0 / params.pellet_rate;
    timer.countdown = interval * (0.5 + hash01(stream_seed(salt, timer.spawned, 4)));
}

/// Sinks pellets steadily and despawns any that reach the tank floor.
fn sink_pellets(
    time: Res<Time>,
    viewport: SketchViewport,
    params: Res<EcoParams>,
    mut pellets: Query<(Entity, &mut Transform), With<Pellet>>,
    mut commands: Commands,
) {
    let Some(view) = viewport.resolve() else {
        return;
    };
    let dt = viewport.delta(&time);
    let speed = PELLET_SINK_SPEED * params.speed_scale;

    for (entity, mut transform) in pellets.iter_mut() {
        transform.translation.y -= speed * dt;
        if transform.translation.y < -view.y * 0.5 {
            commands.entity(entity).despawn();
        }
    }
}

/// Turns queued population events into expanding rings. Purely presentational:
/// it neither reads nor writes simulation state, so the tuned dynamics are
/// identical with markers on or off.
fn spawn_event_markers(
    mut events: ResMut<TankEvents>,
    params: Res<EcoParams>,
    assets: Res<TankAssets>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut commands: Commands,
) {
    let queued = std::mem::take(&mut events.0);
    if !params.markers {
        return;
    }
    for event in queued {
        let (color, duration, base_alpha, growth) = match event.kind {
            TankEventKind::Born => (Color::srgb(0.72, 1.0, 0.86), 0.55, 0.85, 1.0),
            // White for a kill: achromatic, so it cannot be read as any species.
            TankEventKind::Eaten => (Color::srgb(1.0, 1.0, 1.0), 0.45, 0.95, 1.2),
            TankEventKind::Died => (Color::srgb(0.55, 0.68, 0.85), 0.70, 0.60, 0.8),
        };
        let material = materials.add(ColorMaterial {
            color: color.with_alpha(0.0),
            alpha_mode: AlphaMode2d::Blend,
            ..default()
        });
        commands.spawn((
            Mesh2d(assets.ring_mesh.clone()),
            MeshMaterial2d(material.clone()),
            Transform::from_translation(event.position.extend(1.0)),
            EventPulse {
                age: 0.0,
                duration,
                radius: event.radius * growth,
                base_alpha,
                color,
                material,
            },
        ));
    }
}

/// Expands and fades each event ring, despawning it when its life runs out.
fn animate_event_pulses(
    time: Res<Time>,
    viewport: SketchViewport,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut pulses: Query<(Entity, &mut Transform, &mut EventPulse)>,
    mut commands: Commands,
) {
    let dt = viewport.delta(&time);
    for (entity, mut transform, mut pulse) in pulses.iter_mut() {
        pulse.age += dt;
        if pulse.age >= pulse.duration {
            commands.entity(entity).despawn();
            continue;
        }
        let progress = pulse.age / pulse.duration;
        transform.scale = Vec3::splat(pulse.radius * (0.35 + 0.65 * progress));
        let alpha = pulse.base_alpha * (1.0 - progress).powf(1.5);
        if let Some(mut material) = materials.get_mut(&pulse.material) {
            material.color = pulse.color.with_alpha(alpha);
        }
    }
}

/// Keeps the corner readout current with per-species and pellet counts.
fn update_stats(
    time: Res<Time>,
    fish: Query<&Fish>,
    pellets: Query<(), With<Pellet>>,
    mut stats: Query<&mut Text, With<StatsText>>,
) {
    let Ok(mut text) = stats.single_mut() else {
        return;
    };
    let mut counts = [0usize; SPECIES_COUNTS.len()];
    for fish in fish.iter() {
        counts[fish.species] += 1;
    }
    text.0 = format!(
        "{:>4.0}s  cyan {:>3}  lime {:>3}  orange {:>3}  magenta {:>2}  pellets {:>2}",
        time.elapsed_secs(),
        counts[0],
        counts[1],
        counts[2],
        counts[3],
        pellets.iter().count(),
    );
}

/// Prints population counts every 150 simulation frames and, at the end of a
/// headless run, one machine-readable `RESULT` line per run so sweep tools
/// can aggregate equilibrium-search outcomes across parallel processes.
fn log_population(
    viewport: SketchViewport,
    params: Res<EcoParams>,
    stats: Res<EcoStats>,
    time: Res<Time>,
    fish: Query<&Fish>,
    pellets: Query<(), With<Pellet>>,
    mut track: Local<RunTrack>,
) {
    track.frames += 1;
    track.sim_time += viewport.delta(&time);
    if !track.frames.is_multiple_of(150) {
        return;
    }
    let mut counts = [0usize; SPECIES_COUNTS.len()];
    for fish in fish.iter() {
        counts[fish.species] += 1;
    }
    let total = fish.iter().len();
    track.observe(counts, total);

    println!(
        "[pop] sim={:>5.0}s cyan {:>4} lime {:>4} orange {:>4} magenta {:>3} pellets {:>3} total {:>4}",
        track.sim_time,
        counts[0],
        counts[1],
        counts[2],
        counts[3],
        pellets.iter().count(),
        total,
    );

    let Some(recording) = viewport.recording.as_deref() else {
        return;
    };
    if track.frames < recording.max_frames {
        return;
    }
    let extinct: Vec<String> = NAMES
        .iter()
        .enumerate()
        .filter(|(level, _)| track.extinct_seen[*level])
        .map(|(level, name)| format!("{name}@{:.0}", track.extinct_at[level]))
        .collect();
    println!(
        "RESULT seed={} rate={:.3} drain={:.2} catch={:.2} lifespan={:.2} flee={:.2} hunt={:.2} reach={:.2} fear_gate={:.2} pursuit={:.2} hunger={:.2} founders={:.2} counts={}/{}/{}/{} immigration={} duration={:.0} cyan_min={} cyan_end={} lime_min={} lime_end={} orange_min={} orange_end={} magenta_min={} magenta_end={} pellets_end={} total_max={} births={}/{}/{}/{} catches={}/{}/{}/{} attempts={}/{}/{}/{} stocked={}/{}/{}/{} aged={} starved={} extinct={}",
        params.seed,
        params.pellet_rate,
        params.drain_scale,
        params.catch_scale,
        params.lifespan_scale,
        params.flee_scale,
        params.hunt_scale,
        params.reach_scale,
        params.fear_gate_scale,
        params.pursuit_scale,
        params.hunger_scale,
        params.founders_scale,
        params.founder_counts.unwrap_or(SPECIES_COUNTS)[0],
        params.founder_counts.unwrap_or(SPECIES_COUNTS)[1],
        params.founder_counts.unwrap_or(SPECIES_COUNTS)[2],
        params.founder_counts.unwrap_or(SPECIES_COUNTS)[3],
        params.immigration_floor,
        recording.max_frames as f32 * recording.fixed_dt,
        track.mins[0],
        counts[0],
        track.mins[1],
        counts[1],
        track.mins[2],
        counts[2],
        track.mins[3],
        counts[3],
        pellets.iter().count(),
        track.total_max,
        stats.births[0],
        stats.births[1],
        stats.births[2],
        stats.births[3],
        stats.catches[0],
        stats.catches[1],
        stats.catches[2],
        stats.catches[3],
        stats.attempts[0],
        stats.attempts[1],
        stats.attempts[2],
        stats.attempts[3],
        stats.stocked[0],
        stats.stocked[1],
        stats.stocked[2],
        stats.stocked[3],
        stats.aged_out,
        stats.starved,
        if extinct.is_empty() {
            "none".to_string()
        } else {
            extinct.join(",")
        },
    );
}

const NAMES: [&str; SPECIES_COUNTS.len()] = ["cyan", "lime", "orange", "magenta"];

/// Running trajectory summary used for the final `RESULT` line.
#[derive(Default)]
struct RunTrack {
    frames: u32,
    sim_time: f32,
    mins: [usize; 4],
    extinct_at: [f32; 4],
    extinct_seen: [bool; 4],
    total_max: usize,
    started: bool,
}

impl RunTrack {
    fn observe(&mut self, counts: [usize; 4], total: usize) {
        if !self.started {
            self.mins = counts;
            self.started = true;
        }
        self.total_max = self.total_max.max(total);
        for (level, &count) in counts.iter().enumerate() {
            self.mins[level] = self.mins[level].min(count);
            if count == 0 && !self.extinct_seen[level] {
                self.extinct_seen[level] = true;
                self.extinct_at[level] = self.sim_time;
            }
        }
    }
}

/// Reflects a fish off each wall it crosses, keeping it inside the tank.
fn bounce_off_walls(position: Vec2, mut velocity: Vec2, half_bounds: Vec2) -> (Vec2, Vec2) {
    let mut position = position;
    if position.x > half_bounds.x {
        position.x = half_bounds.x;
        velocity.x = -velocity.x;
    } else if position.x < -half_bounds.x {
        position.x = -half_bounds.x;
        velocity.x = -velocity.x;
    }
    if position.y > half_bounds.y {
        position.y = half_bounds.y;
        velocity.y = -velocity.y;
    } else if position.y < -half_bounds.y {
        position.y = -half_bounds.y;
        velocity.y = -velocity.y;
    }
    (position, velocity)
}

/// Linear falloff from full push at zero distance to none at the edge of personal space.
fn repulsion_strength(distance: f32, personal_space: f32) -> f32 {
    (1.0 - distance / personal_space).max(0.0)
}

/// Linear fear ramp: 1 right at the predator, 0 at the edge of its fear radius.
fn fear_factor(distance: f32, fear_radius: f32) -> f32 {
    (1.0 - distance / fear_radius).clamp(0.0, 1.0)
}

/// Advances one fish's dart state machine.
///
/// Returns the next darting state, elapsed burst time, retrigger timer, burst
/// count, and this frame's burst envelope (1 = full acceleration).
fn advance_dart(
    fish_id: u32,
    darting: bool,
    mut elapsed: f32,
    mut timer: f32,
    mut count: u32,
    dt: f32,
) -> (bool, f32, f32, u32, f32) {
    if darting {
        elapsed += dt;
        if elapsed >= DART_TOTAL_SECS {
            count += 1;
            let timer = next_dart_delay(fish_id, count);
            return (false, 0.0, timer, count, 0.0);
        }
        return (true, elapsed, timer, count, dart_envelope(elapsed));
    }

    timer -= dt;
    if timer <= 0.0 {
        count += 1;
        let roll = hash01(stream_seed(fish_id, count, 2));
        if roll < DART_PROBABILITY {
            return (true, 0.0, timer, count, 1.0);
        }
        let timer = next_dart_delay(fish_id, count);
        return (false, 0.0, timer, count, 0.0);
    }
    (false, 0.0, timer, count, 0.0)
}

/// Burst envelope: full strength for `DART_FULL_SECS`, then a linear taper to
/// zero over the following `DART_TAPER_SECS`.
fn dart_envelope(elapsed: f32) -> f32 {
    if elapsed < DART_FULL_SECS {
        1.0
    } else {
        ((DART_TOTAL_SECS - elapsed) / DART_TAPER_SECS).clamp(0.0, 1.0)
    }
}

/// Deterministic hash in [0, 1), standing in for an RNG so runs stay reproducible.
fn hash01(seed: u64) -> f32 {
    let mut hash = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    hash = (hash ^ (hash >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    hash = (hash ^ (hash >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    hash ^= hash >> 31;
    ((hash >> 11) as f64 / (1u64 << 53) as f64) as f32
}

fn stream_seed(fish_id: u32, dart_count: u32, salt: u64) -> u64 {
    (u64::from(fish_id) << 32) | (u64::from(dart_count) << 8) | salt
}
fn next_dart_delay(fish_id: u32, dart_count: u32) -> f32 {
    DART_MIN_DELAY + (DART_MAX_DELAY - DART_MIN_DELAY) * hash01(stream_seed(fish_id, dart_count, 1))
}

/// Steers away from each wall once a fish gets inside its species' margin.
fn wall_avoidance(position: Vec2, half_bounds: Vec2, margin: f32) -> Vec2 {
    let mut desired = Vec2::ZERO;
    if position.x < -half_bounds.x + margin {
        desired.x = 1.0;
    }
    if position.x > half_bounds.x - margin {
        desired.x = -1.0;
    }
    if position.y < -half_bounds.y + margin {
        desired.y = 1.0;
    }
    if position.y > half_bounds.y - margin {
        desired.y = -1.0;
    }
    desired.normalize_or_zero()
}

/// Places fish in a repeatable low-discrepancy distribution without an RNG dependency.
fn initial_position(index: u32, viewport: Vec2) -> Vec2 {
    let x = halton(index, 2);
    let y = halton(index, 3);
    Vec2::new((x - 0.5) * viewport.x, (y - 0.5) * viewport.y)
}

/// A triangle pointing along +X, so a fish's rotation directly matches its heading.
fn fish_mesh(body_length: f32) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            [body_length, 0.0, 0.0],
            [-body_length * 0.5, body_length * 0.4, 0.0],
            [-body_length * 0.5, -body_length * 0.4, 0.0],
        ],
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 3]);
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[0.5, 0.0], [1.0, 1.0], [0.0, 1.0]],
    );
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wall_avoidance_pushes_toward_the_tank_interior() {
        let half_bounds = Vec2::splat(400.0);
        let margin = 50.0;

        assert_eq!(
            wall_avoidance(Vec2::new(-380.0, 0.0), half_bounds, margin),
            Vec2::X
        );
        assert_eq!(
            wall_avoidance(Vec2::new(0.0, 380.0), half_bounds, margin),
            -Vec2::Y
        );
        assert_eq!(
            wall_avoidance(Vec2::new(-395.0, 395.0), half_bounds, margin),
            Vec2::new(1.0, -1.0).normalize()
        );
        assert_eq!(
            wall_avoidance(Vec2::new(0.0, 0.0), half_bounds, margin),
            Vec2::ZERO
        );
    }

    #[test]
    fn repulsion_peaks_at_contact_and_ends_at_personal_space() {
        let space = 26.0;
        assert_eq!(repulsion_strength(0.0, space), 1.0);
        assert!((repulsion_strength(space * 0.5, space) - 0.5).abs() < 1.0e-6);
        assert_eq!(repulsion_strength(space, space), 0.0);
        assert_eq!(repulsion_strength(space * 2.0, space), 0.0);
    }

    #[test]
    fn fish_bounce_off_the_tank_walls() {
        let half_bounds = Vec2::splat(400.0);

        let (position, velocity) =
            bounce_off_walls(Vec2::new(401.0, 0.0), Vec2::new(50.0, -20.0), half_bounds);
        assert_eq!(position, Vec2::new(400.0, 0.0));
        assert_eq!(velocity, Vec2::new(-50.0, -20.0));

        let (position, velocity) =
            bounce_off_walls(Vec2::new(0.0, -420.0), Vec2::new(10.0, -30.0), half_bounds);
        assert_eq!(position, Vec2::new(0.0, -400.0));
        assert_eq!(velocity, Vec2::new(10.0, 30.0));

        let (position, velocity) =
            bounce_off_walls(Vec2::new(0.0, 0.0), Vec2::new(10.0, -30.0), half_bounds);
        assert_eq!(position, Vec2::new(0.0, 0.0));
        assert_eq!(velocity, Vec2::new(10.0, -30.0));
    }

    #[test]
    fn fear_peaks_at_the_predator_and_ends_at_its_radius() {
        let radius = 200.0;
        assert_eq!(fear_factor(0.0, radius), 1.0);
        assert!((fear_factor(radius * 0.5, radius) - 0.5).abs() < 1.0e-6);
        assert_eq!(fear_factor(radius, radius), 0.0);
        assert_eq!(fear_factor(radius * 2.0, radius), 0.0);
    }

    #[test]
    fn eat_distance_is_the_contact_radius_of_both_bodies() {
        assert_eq!(eat_distance(9.0, 5.0), 7.0);
        assert_eq!(eat_distance(26.0, 14.0), 20.0);
    }

    #[test]
    fn dart_bursts_hold_then_taper_off() {
        assert_eq!(dart_envelope(0.0), 1.0);
        assert_eq!(dart_envelope(DART_FULL_SECS * 0.999), 1.0);
        assert_eq!(dart_envelope(DART_FULL_SECS + 1.0), 0.5);
        assert_eq!(dart_envelope(DART_TOTAL_SECS), 0.0);
    }

    #[test]
    fn darts_end_after_the_full_and_taper_duration() {
        let (darting, elapsed, timer, count, factor) =
            advance_dart(7, true, DART_TOTAL_SECS - 0.001, 0.0, 3, 1.0 / 60.0);
        assert!(!darting);
        assert_eq!(elapsed, 0.0);
        assert_eq!(count, 4);
        assert!((DART_MIN_DELAY..=DART_MAX_DELAY).contains(&timer));
        assert_eq!(factor, 0.0);
    }

    #[test]
    fn dart_timers_retrigger_within_three_to_five_seconds() {
        let (darting, _, timer, _, _) = advance_dart(7, false, 0.0, 0.0, 0, 1.0 / 60.0);
        if !darting {
            assert!((DART_MIN_DELAY..=DART_MAX_DELAY).contains(&timer));
        }
        assert!((DART_MIN_DELAY..=DART_MAX_DELAY).contains(&next_dart_delay(3, 9)));
        assert!((DART_MIN_DELAY..=DART_MAX_DELAY).contains(&next_dart_delay(11, 2)));
    }

    #[test]
    fn hash_values_are_deterministic_and_in_the_unit_interval() {
        assert_eq!(hash01(42), hash01(42));
        for seed in [0u64, 1, 12_345, u64::MAX / 2] {
            let value = hash01(seed);
            assert!((0.0..1.0).contains(&value));
        }
    }

    #[test]
    fn energy_pyramid_halves_per_trophic_level() {
        // Each fish needs BIRTH_COST=2 meal-energy above its inherited reserve;
        // half of the parent's investment transfers into the newborn, so each
        // trophic level doubles the source-pellet energy required for a birth.
        assert_eq!(BIRTH_THRESHOLD - NEWBORN_ENERGY, BIRTH_COST);
        let mut pellets_per_birth = BIRTH_COST / PELLET_ENERGY;
        for expected in [2.0, 4.0, 8.0, 16.0] {
            assert_eq!(pellets_per_birth, expected);
            pellets_per_birth *= PREY_BODY_ENERGY / PREY_TRANSFER;
        }
    }

    #[test]
    fn eco_scales_apply_to_species_table_and_catch_probability_cap() {
        let params = EcoParams {
            seed: 2,
            pellet_rate: 2.0,
            drain_scale: 3.0,
            catch_scale: 2.0,
            lifespan_scale: 0.5,
            ..EcoParams::default()
        };
        let baseline = SpeciesTable::schools();
        let scaled = SpeciesTable::schools_with(&params);
        for (base, scaled) in baseline.0.iter().zip(scaled.0.iter()) {
            assert_eq!(scaled.energy_drain, base.energy_drain * 3.0);
            assert_eq!(scaled.lifespan, base.lifespan * 0.5);
            assert!(scaled.catch_chance <= 1.0);
        }
        // Level 2 catch chance doubles; the capped levels stay at or below 1.
        assert_eq!(scaled.0[1].catch_chance, baseline.0[1].catch_chance * 2.0);
        assert_eq!(scaled.0[0].catch_chance, 1.0);
    }

    #[test]
    fn demo_profile_scales_geometry_so_distances_stay_proportional() {
        let params = EcoParams::demo();
        let baseline = SpeciesTable::schools();
        let demo = SpeciesTable::schools_with(&params);
        assert!(params.markers);
        assert_eq!(params.founder_counts, Some([46, 22, 18, 6]));
        for (level, (base, scaled)) in baseline.0.iter().zip(demo.0.iter()).enumerate() {
            // Distances hard-coded in pixels must scale with the bodies, or
            // larger fish would overlap, clip walls, and see the same short
            // hunt ranges as small ones.
            assert_eq!(scaled.body_length, base.body_length * 3.0);
            assert_eq!(scaled.neighbor_radius, base.neighbor_radius * 3.0);
            assert_eq!(scaled.personal_space, base.personal_space * 3.0);
            assert_eq!(scaled.wall_margin, base.wall_margin * 3.0);
            assert_eq!(scaled.hunt_radius, base.hunt_radius * 3.0);
            assert_eq!(scaled.fear_radius, base.fear_radius * 3.0);
            let pursuit = if level > 0 { 2.0 } else { 1.0 };
            assert_eq!(scaled.max_speed, base.max_speed * 0.25 * pursuit);
            assert!(scaled.personal_space > scaled.body_length);
        }
        assert_eq!(demo.0[3].body_length, 78.0);
    }

    #[test]
    fn species_table_orders_trophic_levels_by_size_and_sanity() {
        let species = SpeciesTable::schools();
        assert_eq!(species.0.len(), SPECIES_COUNTS.len());
        for (level, config) in species.0.iter().enumerate() {
            assert!(config.lifespan > 0.0);
            assert!(config.hunger_secs > 0.0);
            assert!(config.energy_drain > 0.0);
            assert!((0.0..=1.0).contains(&config.catch_chance));
            assert!(config.repulsion_weight > 0.0);
            assert!(config.personal_space > config.body_length);
            if level == 0 {
                assert_eq!(config.hunt_radius, PELLET_SEEK_RADIUS);
            } else {
                assert!(config.hunt_radius > 0.0);
            }
            assert_eq!(config.fear_radius > 0.0, level + 1 < species.0.len());
            assert_eq!(config.darts, level + 1 < species.0.len());
        }
        for pair in species.0.windows(2) {
            assert!(pair[1].body_length > pair[0].body_length);
        }
    }
}
