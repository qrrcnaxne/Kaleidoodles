//! A kinematic stickman dancer driven by a beat-based pose phrase.

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use bevy::{
    ecs::system::SystemParam, gizmos::config::DefaultGizmoConfigGroup, prelude::*,
    window::PrimaryWindow,
};

mod audio;
mod moves;

use super::common::{simulation_delta, spawn_sketch_camera, viewport_size};
use crate::recording::{Recording, RecordingRenderTarget};
use audio::{MicrophonePlugin, TempoSource};
use moves::{
    MotionSample, Phrase, WalkthroughMove, blend_samples, generate_phrase, pilot_phrase,
    sample_phrase, sample_walkthrough_move, walkthrough_moves,
};

const GENERATED_PHRASE_BEATS: f32 = 256.0;
const TEMPO_LOSS_TRANSITION_SECS: f32 = 1.0;
const MOVE_WALKTHROUGH_GAP_SECS: f32 = 1.0;
const MOVE_WALKTHROUGH_RETURN_SECS: f32 = 0.25;
const MOVE_WALKTHROUGH_GAP_TOTAL_SECS: f32 =
    MOVE_WALKTHROUGH_GAP_SECS + MOVE_WALKTHROUGH_RETURN_SECS;

const UPPER_LEG_LENGTH: f32 = 112.0;
const LOWER_LEG_LENGTH: f32 = 108.0;
const PELVIS_HEIGHT: f32 = 210.0;
const TORSO_LENGTH: f32 = 150.0;
const HEAD_OFFSET: f32 = 28.0;
const HEAD_RADIUS: f32 = 28.0;
const UPPER_ARM_LENGTH: f32 = 88.0;
const LOWER_ARM_LENGTH: f32 = 82.0;
const FOOT_LENGTH: f32 = 18.0;
const SHOULDER_HALF_WIDTH: f32 = 9.0;
const HIP_HALF_WIDTH: f32 = 9.0;
const GIZMO_LINE_WIDTH: f32 = 7.0;
const TORSO_GRADIENT_SEGMENTS: usize = 64;
const HEAD_GRADIENT_SEGMENTS: usize = 96;
const STAGE_DEPTH_RATIO: f32 = 0.16;
const DANCER_FRAME_MARGIN_RATIO: f32 = 0.06;
const DISCO_RAY_COUNT: usize = 12;
const DISCO_RAY_GROUP_COUNT: usize = 3;
const DISCO_RAYS_PER_GROUP: usize = DISCO_RAY_COUNT / DISCO_RAY_GROUP_COUNT;
const DISCO_GROUP_SEQUENCE: [usize; DISCO_RAY_GROUP_COUNT] = [0, 2, 1];
const DISCO_RAY_SPREAD: f32 = 1.5;
const DISCO_GLOBE_RADIUS: f32 = 34.0;
const DISCO_BEAM_ROTATIONS_PER_PHRASE: f32 = 1.0;
const DISCO_GLOBE_ROTATIONS_PER_PHRASE: f32 = 0.5;
const DISCO_RAY_GROUP_BEATS: f64 = 4.0;
const DISCO_FLASH_DURATION_BEATS: f32 = 1.0;
const DISCO_RAY_MIN_ALPHA: f32 = 0.03;
const DISCO_RAY_MAX_ALPHA: f32 = 0.88;
// Initial trail tuning values; adjust after visual review.
const TRAIL_LIFETIME_SECS: f32 = 0.75;
const TRAIL_MIN_STEP: f32 = 0.5;
const TRAIL_MAX_ALPHA: f32 = 0.8;
const LIMB_TRAIL_COUNT: usize = 8;

pub(super) struct DancerPlugin;

#[derive(Resource)]
struct DancePhrase(Phrase);

impl DancePhrase {
    fn from_args() -> Self {
        let mut args = std::env::args().skip(1);
        let mut seed = 0_u64;
        while let Some(arg) = args.next() {
            if arg == "--seed"
                && let Some(value) = args.next().and_then(|v| v.parse().ok())
            {
                seed = value;
            }
        }
        bevy::log::info!("Procedural dance seed {seed}; {GENERATED_PHRASE_BEATS:.0}-beat phrase.");
        Self(generate_phrase(seed, GENERATED_PHRASE_BEATS))
    }
}

impl Plugin for DancerPlugin {
    fn build(&self, app: &mut App) {
        let move_walkthrough = MoveWalkthrough::from_args();
        let phrase = if move_walkthrough.is_enabled() {
            DancePhrase(pilot_phrase())
        } else {
            DancePhrase::from_args()
        };
        app.add_plugins(MicrophonePlugin)
            .insert_resource(ClearColor(Color::srgb(0.018, 0.025, 0.055)))
            .init_resource::<DanceClock>()
            .insert_resource(move_walkthrough)
            .insert_resource(phrase)
            .init_resource::<LimbTrails>()
            .add_systems(PostStartup, (setup, setup_move_readout))
            .add_systems(
                Update,
                (
                    advance_dance_clock,
                    advance_move_walkthrough,
                    draw_dancer,
                    update_move_readout,
                )
                    .chain(),
            );
    }
}

#[derive(Resource)]
struct DanceClock {
    total_beats: f64,
    playing: bool,
    neutral_blend: f32,
}

impl Default for DanceClock {
    fn default() -> Self {
        Self {
            total_beats: 0.0,
            playing: false,
            neutral_blend: 1.0,
        }
    }
}

impl DanceClock {
    fn advance(&mut self, delta_secs: f32, bpm: Option<f32>) {
        match bpm {
            Some(bpm) => {
                self.total_beats += f64::from(delta_secs) * f64::from(bpm) / 60.0;
                self.neutral_blend =
                    (self.neutral_blend - delta_secs / TEMPO_LOSS_TRANSITION_SECS).max(0.0);
                self.playing = true;
            }
            None => {
                self.neutral_blend =
                    (self.neutral_blend + delta_secs / TEMPO_LOSS_TRANSITION_SECS).min(1.0);
                self.playing = false;
            }
        }
    }
}

#[derive(Resource)]
struct MoveWalkthrough {
    enabled: bool,
    moves: Vec<WalkthroughMove>,
    index: usize,
    elapsed_beats: f64,
    gap_remaining_secs: f32,
    flash_elapsed_secs: f32,
    neutral_blend: f32,
}

impl MoveWalkthrough {
    fn from_args() -> Self {
        Self::new(std::env::args().any(|arg| arg == "--move-demo"))
    }

    fn new(enabled: bool) -> Self {
        let moves = if enabled {
            let moves = walkthrough_moves();
            bevy::log::info!(
                "Move walkthrough enabled: {} moves, one-second neutral gaps.",
                moves.len()
            );
            moves
        } else {
            Vec::new()
        };
        Self {
            enabled,
            moves,
            index: 0,
            elapsed_beats: 0.0,
            gap_remaining_secs: 0.0,
            flash_elapsed_secs: 0.0,
            neutral_blend: 1.0,
        }
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn current_move(&self) -> Option<WalkthroughMove> {
        (self.enabled && self.gap_remaining_secs <= 0.0)
            .then(|| self.moves.get(self.index).copied())
            .flatten()
    }

    fn sample(&self) -> MotionSample {
        if !self.enabled || self.moves.is_empty() {
            return MotionSample::NEUTRAL;
        }
        let cue = self.moves[self.index];
        let sample = if self.gap_remaining_secs > 0.0 {
            let ending = sample_walkthrough_move(cue, 1.0);
            let returned = smoothstep(
                (MOVE_WALKTHROUGH_GAP_TOTAL_SECS - self.gap_remaining_secs)
                    / MOVE_WALKTHROUGH_RETURN_SECS,
            );
            blend_samples(ending, MotionSample::NEUTRAL, returned)
        } else {
            sample_walkthrough_move(cue, (self.elapsed_beats / f64::from(cue.beats)) as f32)
        };
        blend_samples(
            sample,
            MotionSample::NEUTRAL,
            smoothstep(self.neutral_blend),
        )
    }

    fn number_is_bright(&self) -> bool {
        self.flash_elapsed_secs.rem_euclid(1.0) < 0.5
    }

    fn advance(&mut self, delta: f32, bpm: Option<f32>) {
        if !self.enabled || self.moves.is_empty() {
            return;
        }
        self.neutral_blend = if bpm.is_some() {
            (self.neutral_blend - delta / TEMPO_LOSS_TRANSITION_SECS).max(0.0)
        } else {
            (self.neutral_blend + delta / TEMPO_LOSS_TRANSITION_SECS).min(1.0)
        };

        let mut remaining = delta.max(0.0);
        while remaining > 0.0 {
            if self.gap_remaining_secs > 0.0 {
                let consumed = remaining.min(self.gap_remaining_secs);
                self.gap_remaining_secs -= consumed;
                remaining -= consumed;
                if self.gap_remaining_secs > 1.0e-6 {
                    break;
                }
                self.gap_remaining_secs = 0.0;
                if remaining < 1.0e-6 {
                    remaining = 0.0;
                }
                self.index = (self.index + 1) % self.moves.len();
                self.elapsed_beats = 0.0;
                self.flash_elapsed_secs = 0.0;
                continue;
            }

            let Some(bpm) = bpm.filter(|bpm| bpm.is_finite() && *bpm > 0.0) else {
                self.flash_elapsed_secs += remaining;
                break;
            };
            let cue = self.moves[self.index];
            let beats_left = (f64::from(cue.beats) - self.elapsed_beats).max(0.0);
            let seconds_to_finish = (beats_left * 60.0 / f64::from(bpm)) as f32;
            if remaining < seconds_to_finish {
                self.elapsed_beats += f64::from(remaining) * f64::from(bpm) / 60.0;
                self.flash_elapsed_secs += remaining;
                break;
            }
            remaining -= seconds_to_finish;
            self.flash_elapsed_secs += seconds_to_finish;
            self.elapsed_beats = f64::from(cue.beats);
            self.gap_remaining_secs = MOVE_WALKTHROUGH_GAP_TOTAL_SECS;
        }
    }
}

#[derive(Component)]
struct MoveNumberText;

#[derive(Component)]
struct MoveNameText;

type MoveNumberQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Text, &'static mut TextColor),
    (With<MoveNumberText>, Without<MoveNameText>),
>;
type MoveNameQuery<'w, 's> =
    Query<'w, 's, &'static mut Text, (With<MoveNameText>, Without<MoveNumberText>)>;

#[derive(SystemParam)]
struct DancerMotion<'w> {
    clock: Res<'w, DanceClock>,
    walkthrough: Res<'w, MoveWalkthrough>,
    phrase: Res<'w, DancePhrase>,
    time: Res<'w, Time>,
    recording: Option<Res<'w, Recording>>,
}

#[derive(Resource, Default)]
struct LimbTrails([TrailTrack; LIMB_TRAIL_COUNT]);

#[derive(Component)]
struct DiscoGlobe;

#[derive(Default)]
struct TrailTrack {
    previous: Option<Vec2>,
    segments: Vec<TrailSegment>,
}

#[derive(Clone, Copy)]
struct TrailSegment {
    start: Vec2,
    end: Vec2,
    age_secs: f32,
}

impl TrailTrack {
    fn advance(&mut self, position: Vec2, delta_secs: f32) {
        for segment in &mut self.segments {
            segment.age_secs += delta_secs;
        }
        self.segments
            .retain(|segment| segment.age_secs < TRAIL_LIFETIME_SECS);

        if let Some(previous) = self.previous
            && previous.distance(position) >= TRAIL_MIN_STEP
        {
            self.segments.push(TrailSegment {
                start: previous,
                end: position,
                age_secs: 0.0,
            });
        }
        self.previous = Some(position);
    }
}

#[derive(Clone, Copy)]
struct LegJoints {
    knee: Vec2,
    foot: Vec2,
    toe: Vec2,
}

#[derive(Clone, Copy)]
struct FigureJoints {
    pelvis: Vec2,
    facing: f32,
    shoulder_center: Vec2,
    left_hip: Vec2,
    right_hip: Vec2,
    left_shoulder: Vec2,
    right_shoulder: Vec2,
    left_elbow: Vec2,
    left_hand: Vec2,
    right_elbow: Vec2,
    right_hand: Vec2,
    left_knee: Vec2,
    left_foot: Vec2,
    left_toe: Vec2,
    right_knee: Vec2,
    right_foot: Vec2,
    right_toe: Vec2,
    head_center: Vec2,
    head_direction: Vec2,
    neck_bottom: Vec2,
}

fn setup(
    mut commands: Commands,
    mut gizmo_configs: ResMut<GizmoConfigStore>,
    recording: Option<Res<Recording>>,
    render_target: Option<Res<RecordingRenderTarget>>,
    window: Option<Single<&Window, With<PrimaryWindow>>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let window_size = window
        .as_ref()
        .map(|window| Vec2::new(window.width(), window.height()));
    if viewport_size(recording.as_deref(), window_size).is_none() {
        return;
    }

    spawn_sketch_camera(&mut commands, render_target.as_deref());
    commands.spawn((
        Mesh2d(meshes.add(Circle::new(1.0))),
        MeshMaterial2d(materials.add(Color::srgb(0.27, 0.30, 0.39))),
        Transform::default(),
        DiscoGlobe,
    ));
    let (config, _) = gizmo_configs.config_mut::<DefaultGizmoConfigGroup>();
    config.line.width = GIZMO_LINE_WIDTH;
}

fn setup_move_readout(mut commands: Commands, walkthrough: Res<MoveWalkthrough>) {
    if !walkthrough.enabled {
        return;
    }
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(24.0),
            left: Val::Px(24.0),
            ..default()
        },
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(46.0),
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.82, 0.2)),
        MoveNumberText,
    ));
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(78.0),
            left: Val::Px(24.0),
            ..default()
        },
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(22.0),
            ..default()
        },
        TextColor(Color::WHITE),
        MoveNameText,
    ));
}

fn advance_dance_clock(
    time: Res<Time>,
    recording: Option<Res<Recording>>,
    tempo: Res<TempoSource>,
    mut clock: ResMut<DanceClock>,
) {
    let delta = simulation_delta(recording.as_deref(), time.delta_secs());
    clock.advance(delta, tempo.bpm);
}

fn advance_move_walkthrough(
    time: Res<Time>,
    recording: Option<Res<Recording>>,
    tempo: Res<TempoSource>,
    mut walkthrough: ResMut<MoveWalkthrough>,
) {
    let delta = simulation_delta(recording.as_deref(), time.delta_secs());
    walkthrough.advance(delta, tempo.bpm);
}

fn update_move_readout(
    walkthrough: Res<MoveWalkthrough>,
    mut number: MoveNumberQuery,
    mut name: MoveNameQuery,
) {
    let cue = walkthrough.current_move();
    if let Ok((mut text, mut color)) = number.single_mut() {
        if cue.is_some() {
            text.0 = format!("{:02}", walkthrough.index + 1);
            color.0 = if walkthrough.number_is_bright() {
                Color::srgb(1.0, 0.82, 0.2)
            } else {
                Color::srgb(0.42, 0.34, 0.14)
            };
        } else {
            text.0.clear();
        }
    }
    if let Ok(mut text) = name.single_mut() {
        text.0 = cue.map_or_else(String::new, |cue| {
            format!("Dance move {} — {}", walkthrough.index + 1, cue.name)
        });
    }
}

fn draw_dancer(
    mut gizmos: Gizmos,
    motion: DancerMotion,
    window: Option<Single<&Window, With<PrimaryWindow>>>,
    mut trails: ResMut<LimbTrails>,
    mut globe: Query<&mut Transform, With<DiscoGlobe>>,
) {
    let window_size = window
        .as_ref()
        .map(|window| Vec2::new(window.width(), window.height()));
    let Some(viewport) = viewport_size(motion.recording.as_deref(), window_size) else {
        return;
    };

    let scale = (viewport.y / 850.0).clamp(0.8, 1.8);
    let stage_back_y = -viewport.y * 0.25;
    let stage_depth = viewport.y * STAGE_DEPTH_RATIO;
    let floor_y = stage_back_y - stage_depth * 0.5;
    let globe_center = Vec2::new(0.0, viewport.y * 0.36);
    let globe_radius = DISCO_GLOBE_RADIUS * scale;
    for mut transform in &mut globe {
        transform.translation = globe_center.extend(0.0);
        transform.scale = Vec3::splat(globe_radius);
    }
    let total_beats = motion.clock.total_beats;
    let phrase_beats = motion.phrase.0.beats;
    let phrase_beat = total_beats.rem_euclid(f64::from(phrase_beats)) as f32;
    let sample = if motion.walkthrough.enabled {
        motion.walkthrough.sample()
    } else {
        pose_at_clock(&motion.clock, &motion.phrase.0)
    };
    let (joints, scale) = fit_figure_to_viewport(
        figure_joints(sample, scale, floor_y),
        scale,
        viewport,
        floor_y,
    );
    let endpoints = limb_endpoints(joints);
    let delta = simulation_delta(motion.recording.as_deref(), motion.time.delta_secs());
    for (trail, endpoint) in trails.0.iter_mut().zip(endpoints) {
        trail.advance(endpoint, delta);
    }

    draw_stage(&mut gizmos, viewport, stage_back_y);
    if motion.clock.playing {
        draw_disco_rays(
            &mut gizmos,
            viewport,
            globe_center,
            globe_radius,
            stage_back_y,
            phrase_beat,
            phrase_beats,
            total_beats,
        );
    }
    draw_disco_globe(
        &mut gizmos,
        globe_center,
        globe_radius,
        phrase_beat,
        phrase_beats,
    );
    draw_limb_trails(&mut gizmos, &trails);
    draw_figure(&mut gizmos, joints, scale);
}

impl FigureJoints {
    fn positions(self) -> [Vec2; 18] {
        [
            self.pelvis,
            self.shoulder_center,
            self.left_hip,
            self.right_hip,
            self.left_shoulder,
            self.right_shoulder,
            self.left_elbow,
            self.left_hand,
            self.right_elbow,
            self.right_hand,
            self.left_knee,
            self.left_foot,
            self.left_toe,
            self.right_knee,
            self.right_foot,
            self.right_toe,
            self.head_center,
            self.neck_bottom,
        ]
    }

    fn map_positions(&mut self, mut map: impl FnMut(Vec2) -> Vec2) {
        for point in [
            &mut self.pelvis,
            &mut self.shoulder_center,
            &mut self.left_hip,
            &mut self.right_hip,
            &mut self.left_shoulder,
            &mut self.right_shoulder,
            &mut self.left_elbow,
            &mut self.left_hand,
            &mut self.right_elbow,
            &mut self.right_hand,
            &mut self.left_knee,
            &mut self.left_foot,
            &mut self.left_toe,
            &mut self.right_knee,
            &mut self.right_foot,
            &mut self.right_toe,
            &mut self.head_center,
            &mut self.neck_bottom,
        ] {
            *point = map(*point);
        }
    }
}

/// Keep the complete silhouette in frame, translating the whole rig together.
/// If an unusually wide stance cannot fit, uniformly shrink it around the floor.
fn fit_figure_to_viewport(
    mut joints: FigureJoints,
    scale: f32,
    viewport: Vec2,
    floor_y: f32,
) -> (FigureJoints, f32) {
    let positions = joints.positions();
    let min_x = positions
        .iter()
        .map(|point| point.x)
        .fold(f32::INFINITY, f32::min);
    let max_x = positions
        .iter()
        .map(|point| point.x)
        .fold(f32::NEG_INFINITY, f32::max);
    // Include head outline, facial cue, joint markers and stroke thickness.
    let padding = HEAD_RADIUS * 1.1 * scale;
    let half_width = viewport.x * (0.5 - DANCER_FRAME_MARGIN_RATIO);
    let fit = ((half_width * 2.0 - GIZMO_LINE_WIDTH * 2.0).max(1.0)
        / (max_x - min_x + padding * 2.0))
        .min(1.0);
    let left = (min_x - padding) * fit - GIZMO_LINE_WIDTH;
    let right = (max_x + padding) * fit + GIZMO_LINE_WIDTH;
    let offset = 0.0_f32.clamp(
        -half_width - left,
        (half_width - right).max(-half_width - left),
    );
    joints.map_positions(|point| {
        Vec2::new(point.x * fit + offset, floor_y + (point.y - floor_y) * fit)
    });
    (joints, scale * fit)
}

fn figure_joints(sample: MotionSample, scale: f32, floor_y: f32) -> FigureJoints {
    let pose = sample.state.pose;
    let facing = facing_projection(sample.state.turn_angle);
    // Ankle lean shifts the pelvis in an arc and tilts the whole upper body.
    let lean_offset = pose.ankle_lean * PELVIS_HEIGHT * scale;
    let pelvis = Vec2::new(
        sample.state.root.x * scale + lean_offset,
        floor_y + (PELVIS_HEIGHT - pose.squat + pose.pelvis_lift + sample.state.root.y) * scale,
    );
    let to_world = |point: Vec2| pelvis + Vec2::new(point.x * facing, point.y);
    let left_hip_local = Vec2::new(-HIP_HALF_WIDTH * scale, 0.0);
    let right_hip_local = Vec2::new(HIP_HALF_WIDTH * scale, 0.0);
    let left_foot_local = foot_points(
        sample.feet[0],
        pose.left_toe_pitch,
        scale,
        floor_y,
        pelvis.y,
        sample.state.root.x,
    );
    let right_foot_local = foot_points(
        sample.feet[1],
        pose.right_toe_pitch,
        scale,
        floor_y,
        pelvis.y,
        sample.state.root.x,
    );
    let left_knee_local = solve_leg(
        left_hip_local,
        left_foot_local.0,
        UPPER_LEG_LENGTH * scale,
        LOWER_LEG_LENGTH * scale,
        1.0,
    );
    let right_knee_local = solve_leg(
        right_hip_local,
        right_foot_local.0,
        UPPER_LEG_LENGTH * scale,
        LOWER_LEG_LENGTH * scale,
        1.0,
    );
    let left_hip = to_world(left_hip_local);
    let right_hip = to_world(right_hip_local);
    let left_foot = to_world(left_foot_local.0);
    let left_toe = to_world(left_foot_local.1);
    let right_foot = to_world(right_foot_local.0);
    let right_toe = to_world(right_foot_local.1);
    let torso_axis = Vec2::from_angle(FRAC_PI_2 + pose.torso_lean + pose.ankle_lean);
    let shoulder_center_local = torso_axis * ((TORSO_LENGTH + pose.chest_pop) * scale);
    let left_shoulder_local = shoulder_center_local
        + Vec2::new(
            -SHOULDER_HALF_WIDTH * scale,
            pose.left_shoulder_lift * scale,
        );
    let right_shoulder_local = shoulder_center_local
        + Vec2::new(
            SHOULDER_HALF_WIDTH * scale,
            pose.right_shoulder_lift * scale,
        );
    let (left_elbow_local, left_hand_local) = arm_joints(
        left_shoulder_local,
        -1.0,
        pose.left_shoulder,
        pose.left_elbow,
        scale,
    );
    let (right_elbow_local, right_hand_local) = arm_joints(
        right_shoulder_local,
        1.0,
        pose.right_shoulder,
        pose.right_elbow,
        scale,
    );
    let head_center_local = shoulder_center_local + torso_axis * (HEAD_OFFSET * scale);
    let neck_bottom_local = head_center_local - torso_axis * (HEAD_RADIUS * scale * 0.8);
    let head_direction_local = Vec2::from_angle(-pose.head_snap);

    FigureJoints {
        pelvis,
        facing,
        shoulder_center: to_world(shoulder_center_local),
        left_hip,
        right_hip,
        left_shoulder: to_world(left_shoulder_local),
        right_shoulder: to_world(right_shoulder_local),
        left_elbow: to_world(left_elbow_local),
        left_hand: to_world(left_hand_local),
        right_elbow: to_world(right_elbow_local),
        right_hand: to_world(right_hand_local),
        left_knee: to_world(left_knee_local),
        left_foot,
        left_toe,
        right_knee: to_world(right_knee_local),
        right_foot,
        right_toe,
        head_center: to_world(head_center_local),
        head_direction: Vec2::new(head_direction_local.x * facing, head_direction_local.y),
        neck_bottom: to_world(neck_bottom_local),
    }
}

fn limb_endpoints(joints: FigureJoints) -> [Vec2; LIMB_TRAIL_COUNT] {
    [
        joints.left_elbow,
        joints.left_hand,
        joints.right_elbow,
        joints.right_hand,
        joints.left_knee,
        joints.left_foot,
        joints.right_knee,
        joints.right_foot,
    ]
}

fn foot_points(
    sample: moves::FootSample,
    pose_pitch: f32,
    scale: f32,
    floor_y: f32,
    pelvis_y: f32,
    root_x: f32,
) -> (Vec2, Vec2) {
    let part_pitch = match sample.part {
        moves::FootPart::Flat => 0.0,
        moves::FootPart::Heel => 0.55,
        moves::FootPart::Ball => -0.45,
        moves::FootPart::Toe => -0.75,
    };
    let pitch = (part_pitch + pose_pitch).clamp(-1.1, 0.8);
    let toe_length = FOOT_LENGTH * scale;
    let ankle_height = (-pitch.sin()).max(0.0) * toe_length;
    let ankle = Vec2::new(
        (sample.position.x - root_x) * scale,
        floor_y + (sample.position.y + sample.lift) * scale + ankle_height - pelvis_y,
    );
    let toe = ankle + Vec2::new(toe_length * pitch.cos(), toe_length * pitch.sin());
    (ankle, toe)
}

fn facing_projection(turn_angle: f32) -> f32 {
    turn_angle.cos()
}

fn draw_limb_trails(gizmos: &mut Gizmos, trails: &LimbTrails) {
    let colors = [
        LimbSegment::LeftUpperArm,
        LimbSegment::LeftLowerArm,
        LimbSegment::RightUpperArm,
        LimbSegment::RightLowerArm,
        LimbSegment::LeftUpperLeg,
        LimbSegment::LeftLowerLeg,
        LimbSegment::RightUpperLeg,
        LimbSegment::RightLowerLeg,
    ];
    for (trail, segment) in trails.0.iter().zip(colors) {
        let base_color = limb_color(segment);
        for line in &trail.segments {
            let color = base_color.with_alpha(trail_alpha(line.age_secs) * TRAIL_MAX_ALPHA);
            gizmos.line_2d(line.start, line.end, color);
        }
    }
}

fn disco_beam_phase(beat: f32, phrase_beats: f32) -> f32 {
    beat * TAU * DISCO_BEAM_ROTATIONS_PER_PHRASE / phrase_beats
}

fn disco_ray_group(total_beats: f64) -> usize {
    let sequence_index = (total_beats / DISCO_RAY_GROUP_BEATS)
        .floor()
        .rem_euclid(DISCO_RAY_GROUP_COUNT as f64) as usize;
    DISCO_GROUP_SEQUENCE[sequence_index]
}

fn disco_beat_pulse(total_beats: f64) -> f32 {
    let phase = total_beats.rem_euclid(DISCO_RAY_GROUP_BEATS) as f32;
    let pulse = (1.0 - phase / DISCO_FLASH_DURATION_BEATS).clamp(0.0, 1.0);
    pulse * pulse * (3.0 - 2.0 * pulse)
}

fn disco_ray_alpha(total_beats: f64, ray_index: usize) -> f32 {
    let ray_group = ray_index / DISCO_RAYS_PER_GROUP;
    if ray_group != disco_ray_group(total_beats) {
        return DISCO_RAY_MIN_ALPHA;
    }

    DISCO_RAY_MIN_ALPHA
        + (DISCO_RAY_MAX_ALPHA - DISCO_RAY_MIN_ALPHA) * disco_beat_pulse(total_beats)
}

fn trail_alpha(age_secs: f32) -> f32 {
    (1.0 - age_secs / TRAIL_LIFETIME_SECS)
        .clamp(0.0, 1.0)
        .powf(1.5)
}

fn draw_stage(gizmos: &mut Gizmos, viewport: Vec2, back_y: f32) {
    let back_half_width = viewport.x * 0.32;
    let front_half_width = viewport.x * 0.46;
    let front_y = back_y - viewport.y * STAGE_DEPTH_RATIO;
    let back_left = Vec2::new(-back_half_width, back_y);
    let back_right = Vec2::new(back_half_width, back_y);
    let front_left = Vec2::new(-front_half_width, front_y);
    let front_right = Vec2::new(front_half_width, front_y);
    // Light lines stay legible against the dancer's dark backdrop.
    let stage_color = Color::srgb(0.72, 0.80, 0.91);

    gizmos.line_2d(back_left, back_right, stage_color);
    gizmos.line_2d(back_left, front_left, stage_color);
    gizmos.line_2d(back_right, front_right, stage_color);
    gizmos.line_2d(front_left, front_right, stage_color);
}

#[allow(
    clippy::too_many_arguments,
    reason = "Drawing parameters describe one disco ray pass"
)]
fn draw_disco_rays(
    gizmos: &mut Gizmos,
    viewport: Vec2,
    globe_center: Vec2,
    globe_radius: f32,
    stage_back_y: f32,
    phrase_beat: f32,
    phrase_beats: f32,
    total_beats: f64,
) {
    let stage_depth = viewport.y * STAGE_DEPTH_RATIO;
    let back_half_width = viewport.x * 0.32;
    let front_half_width = viewport.x * 0.46;
    let beam_phase = disco_beam_phase(phrase_beat, phrase_beats);
    let common_sweep = beam_phase.sin() * 0.22;

    for index in 0..DISCO_RAY_COUNT {
        let fraction = index as f32 / (DISCO_RAY_COUNT - 1) as f32;
        let ray_wobble = (beam_phase * 2.0 + index as f32 * 1.7).sin() * 0.13;
        let angle = -FRAC_PI_2 + (fraction - 0.5) * DISCO_RAY_SPREAD + common_sweep + ray_wobble;
        let direction = Vec2::from_angle(angle);
        let depth_fraction = 0.15 + 0.75 * (0.5 + 0.5 * (beam_phase + index as f32 * 0.83).sin());
        let target_y = stage_back_y - stage_depth * depth_fraction;
        let target_half_width =
            back_half_width + (front_half_width - back_half_width) * depth_fraction;
        let distance_to_stage = (target_y - globe_center.y) / direction.y;
        let distance_to_edge = target_half_width / direction.x.abs().max(0.01);
        let distance = distance_to_stage.min(distance_to_edge);
        if distance <= globe_radius {
            continue;
        }

        let start = globe_center + direction * (globe_radius * 0.96);
        let end = globe_center + direction * distance;
        let hue = ((fraction + phrase_beat / phrase_beats).rem_euclid(1.0)) * 360.0;
        let alpha = disco_ray_alpha(total_beats, index);
        gizmos.line_2d(start, end, Color::hsl(hue, 1.0, 0.62).with_alpha(alpha));
    }
}

fn draw_disco_globe(gizmos: &mut Gizmos, center: Vec2, radius: f32, beat: f32, phrase_beats: f32) {
    let rotation = beat * TAU * DISCO_GLOBE_ROTATIONS_PER_PHRASE / phrase_beats;
    let bright_facet = Color::srgb(0.90, 0.94, 1.0);
    let dim_facet = Color::srgb(0.47, 0.55, 0.68);

    gizmos
        .circle_2d(
            Isometry2d::from_translation(center),
            radius,
            Color::srgb(0.98, 0.99, 1.0),
        )
        .resolution(64);

    for row in -2..=2 {
        let offset_y = row as f32 * radius * 0.34;
        let half_width = (radius * radius - offset_y * offset_y).sqrt();
        let color = if row % 2 == 0 {
            bright_facet
        } else {
            dim_facet
        };
        gizmos.line_2d(
            center + Vec2::new(-half_width, offset_y),
            center + Vec2::new(half_width, offset_y),
            color,
        );
    }

    for meridian in 0..5 {
        let longitude = meridian as f32 * PI / 5.0;
        let half_width = radius * (0.16 + 0.84 * (rotation + longitude).sin().abs());
        let color = if meridian % 2 == 0 {
            bright_facet
        } else {
            dim_facet
        };
        for step in 0..32 {
            let start_angle = TAU * step as f32 / 32.0;
            let end_angle = TAU * (step + 1) as f32 / 32.0;
            let start =
                center + Vec2::new(half_width * start_angle.cos(), radius * start_angle.sin());
            let end = center + Vec2::new(half_width * end_angle.cos(), radius * end_angle.sin());
            gizmos.line_2d(start, end, color);
        }
    }

    let highlight = center + Vec2::new(-radius * 0.32, radius * 0.32);
    gizmos.circle_2d(
        Isometry2d::from_translation(highlight),
        radius * 0.12,
        Color::WHITE,
    );
}

fn draw_figure(gizmos: &mut Gizmos, joints: FigureJoints, scale: f32) {
    let torso_color = Color::srgb(0.62, 0.70, 0.80);

    draw_leg(
        gizmos,
        joints.left_hip,
        LegJoints {
            knee: joints.left_knee,
            foot: joints.left_foot,
            toe: joints.left_toe,
        },
        limb_color(LimbSegment::LeftUpperLeg),
        limb_color(LimbSegment::LeftLowerLeg),
        scale,
    );
    draw_segment(
        gizmos,
        joints.left_shoulder,
        joints.left_elbow,
        limb_color(LimbSegment::LeftUpperArm),
    );
    draw_segment(
        gizmos,
        joints.left_elbow,
        joints.left_hand,
        limb_color(LimbSegment::LeftLowerArm),
    );
    draw_joint(gizmos, joints.left_elbow, 7.0 * scale, Color::WHITE);

    draw_rainbow_segment(
        gizmos,
        joints.shoulder_center,
        joints.pelvis,
        TORSO_GRADIENT_SEGMENTS,
    );
    draw_segment(
        gizmos,
        joints.left_shoulder,
        joints.shoulder_center,
        rainbow_color(0.0),
    );
    draw_segment(
        gizmos,
        joints.right_shoulder,
        joints.shoulder_center,
        rainbow_color(0.0),
    );
    draw_joint(gizmos, joints.pelvis, 8.0 * scale, Color::WHITE);

    draw_segment(
        gizmos,
        joints.right_shoulder,
        joints.right_elbow,
        limb_color(LimbSegment::RightUpperArm),
    );
    draw_segment(
        gizmos,
        joints.right_elbow,
        joints.right_hand,
        limb_color(LimbSegment::RightLowerArm),
    );
    draw_joint(gizmos, joints.right_elbow, 7.0 * scale, Color::WHITE);
    draw_leg(
        gizmos,
        joints.right_hip,
        LegJoints {
            knee: joints.right_knee,
            foot: joints.right_foot,
            toe: joints.right_toe,
        },
        limb_color(LimbSegment::RightUpperLeg),
        limb_color(LimbSegment::RightLowerLeg),
        scale,
    );

    draw_joint(gizmos, joints.left_hip, 7.0 * scale, Color::WHITE);
    draw_joint(gizmos, joints.right_hip, 7.0 * scale, Color::WHITE);
    draw_joint(gizmos, joints.left_shoulder, 7.0 * scale, Color::WHITE);
    draw_joint(gizmos, joints.right_shoulder, 7.0 * scale, Color::WHITE);
    draw_segment(
        gizmos,
        joints.shoulder_center,
        joints.neck_bottom,
        rainbow_color(0.0),
    );
    draw_rainbow_head(
        gizmos,
        joints.head_center,
        HEAD_RADIUS * scale,
        joints.facing,
    );
    draw_segment(
        gizmos,
        joints.head_center + joints.head_direction * (HEAD_RADIUS * scale * 0.8),
        joints.head_center
            + joints.head_direction * (HEAD_RADIUS * scale * 1.08)
            + Vec2::new(0.0, -3.0 * scale),
        torso_color,
    );
}

fn draw_rainbow_segment(gizmos: &mut Gizmos, start: Vec2, end: Vec2, segments: usize) {
    for index in 0..segments {
        let start_t = index as f32 / segments as f32;
        let end_t = (index + 1) as f32 / segments as f32;
        let color = rainbow_color((start_t + end_t) * 0.5);
        gizmos.line_2d(start.lerp(end, start_t), start.lerp(end, end_t), color);
    }
}

fn draw_rainbow_head(gizmos: &mut Gizmos, center: Vec2, radius: f32, facing: f32) {
    let horizontal_scale = 0.2 + 0.8 * facing.abs().clamp(0.0, 1.0);
    for index in 0..HEAD_GRADIENT_SEGMENTS {
        let start_t = index as f32 / HEAD_GRADIENT_SEGMENTS as f32;
        let end_t = (index + 1) as f32 / HEAD_GRADIENT_SEGMENTS as f32;
        let start_angle = TAU * start_t - FRAC_PI_2;
        let end_angle = TAU * end_t - FRAC_PI_2;
        let color = rainbow_color((start_t + end_t) * 0.5);
        let start = center
            + Vec2::new(
                start_angle.cos() * radius * horizontal_scale,
                start_angle.sin() * radius,
            );
        let end = center
            + Vec2::new(
                end_angle.cos() * radius * horizontal_scale,
                end_angle.sin() * radius,
            );
        gizmos.line_2d(start, end, color);
    }
}

fn rainbow_color(progress: f32) -> Color {
    Color::hsl(rainbow_hue(progress), 0.95, 0.62)
}

fn rainbow_hue(progress: f32) -> f32 {
    progress.rem_euclid(1.0) * 360.0
}

fn draw_leg(
    gizmos: &mut Gizmos,
    hip: Vec2,
    leg: LegJoints,
    upper_color: Color,
    lower_color: Color,
    scale: f32,
) {
    draw_segment(gizmos, hip, leg.knee, upper_color);
    draw_segment(gizmos, leg.knee, leg.foot, lower_color);
    draw_segment(gizmos, leg.foot, leg.toe, lower_color);
    draw_joint(gizmos, leg.knee, 7.0 * scale, Color::WHITE);
}

#[derive(Clone, Copy)]
enum LimbSegment {
    LeftUpperArm,
    LeftLowerArm,
    RightUpperArm,
    RightLowerArm,
    LeftUpperLeg,
    LeftLowerLeg,
    RightUpperLeg,
    RightLowerLeg,
}

fn limb_hue(segment: LimbSegment) -> f32 {
    match segment {
        LimbSegment::LeftUpperArm => 0.0,
        LimbSegment::LeftLowerArm => 180.0,
        LimbSegment::RightUpperArm => 90.0,
        LimbSegment::RightLowerArm => 270.0,
        LimbSegment::LeftUpperLeg => 45.0,
        LimbSegment::LeftLowerLeg => 225.0,
        LimbSegment::RightUpperLeg => 135.0,
        LimbSegment::RightLowerLeg => 315.0,
    }
}

fn limb_color(segment: LimbSegment) -> Color {
    Color::hsl(limb_hue(segment), 1.0, 0.62)
}

fn draw_segment(gizmos: &mut Gizmos, start: Vec2, end: Vec2, color: Color) {
    gizmos.line_2d(start, end, color);
}

fn draw_joint(gizmos: &mut Gizmos, center: Vec2, radius: f32, color: Color) {
    gizmos.circle_2d(Isometry2d::from_translation(center), radius, color);
}

fn arm_joints(shoulder: Vec2, side: f32, spread: f32, elbow_bend: f32, scale: f32) -> (Vec2, Vec2) {
    let upper_angle = -FRAC_PI_2 + side * spread;
    let lower_angle = upper_angle - side * elbow_bend;
    let elbow = shoulder + Vec2::from_angle(upper_angle) * (UPPER_ARM_LENGTH * scale);
    let hand = elbow + Vec2::from_angle(lower_angle) * (LOWER_ARM_LENGTH * scale);
    (elbow, hand)
}

fn solve_leg(hip: Vec2, foot: Vec2, upper_length: f32, lower_length: f32, bend: f32) -> Vec2 {
    let offset = foot - hip;
    let actual_distance = offset.length();
    if actual_distance <= 1.0e-5 {
        return hip;
    }

    let direction = offset / actual_distance;
    let mut upper = upper_length;
    let mut lower = lower_length;
    let maximum_reach = upper + lower;
    let minimum_reach = (upper - lower).abs();
    if actual_distance > maximum_reach {
        // Stretch both segments proportionally rather than drawing a disconnected overreach.
        let stretch = actual_distance / maximum_reach;
        upper *= stretch;
        lower *= stretch;
    } else if actual_distance < minimum_reach {
        // Near a folded singularity, shorten both segments to keep a finite triangle.
        upper = actual_distance * 0.5;
        lower = actual_distance * 0.5;
    }

    let distance = actual_distance.max(1.0e-5);
    let along = (upper.powi(2) - lower.powi(2) + distance.powi(2)) / (2.0 * distance);
    let perpendicular = (upper.powi(2) - along.powi(2)).max(0.0).sqrt();
    hip + direction * along + direction.perp() * (perpendicular * bend)
}

fn pose_at_clock(clock: &DanceClock, phrase: &Phrase) -> MotionSample {
    let sample = sample_phrase(phrase, clock.total_beats);
    blend_samples(
        sample,
        MotionSample::NEUTRAL,
        smoothstep(clock.neutral_blend),
    )
}

fn smoothstep(progress: f32) -> f32 {
    let progress = progress.clamp(0.0, 1.0);
    progress * progress * (3.0 - 2.0 * progress)
}

#[cfg(test)]
mod tests {
    use super::moves::PILOT_PHRASE_BEATS;
    use super::*;

    #[test]
    fn transitions_have_smoothstep_endpoints_and_midpoint() {
        assert_eq!(smoothstep(0.0), 0.0);
        assert_eq!(smoothstep(0.5), 0.5);
        assert_eq!(smoothstep(1.0), 1.0);
    }

    #[test]
    fn inverse_kinematics_preserves_bone_lengths_for_reachable_targets() {
        let hip = Vec2::new(-9.0, 205.0);
        let foot = Vec2::new(-48.0, 0.0);
        let knee = solve_leg(hip, foot, UPPER_LEG_LENGTH, LOWER_LEG_LENGTH, 1.0);
        assert!((knee.distance(hip) - UPPER_LEG_LENGTH).abs() < 1.0e-3);
        assert!((foot.distance(knee) - LOWER_LEG_LENGTH).abs() < 1.0e-3);
    }

    #[test]
    fn both_side_view_knees_bend_toward_the_dancers_forward_direction() {
        let joints = figure_joints(MotionSample::NEUTRAL, 1.0, 0.0);
        assert!(joints.left_knee.x > joints.left_hip.x);
        assert!(joints.right_knee.x > joints.right_hip.x);
    }

    #[test]
    fn unreachable_and_singular_leg_targets_remain_finite_and_connected() {
        let hip = Vec2::new(0.0, 100.0);
        for foot in [hip, Vec2::new(0.0, -500.0)] {
            let knee = solve_leg(hip, foot, UPPER_LEG_LENGTH, LOWER_LEG_LENGTH, 1.0);
            assert!(knee.is_finite());
            assert!(knee.distance(hip).is_finite());
            assert!(knee.distance(foot).is_finite());
        }
        let far_foot = Vec2::new(0.0, -500.0);
        let stretched_knee = solve_leg(hip, far_foot, UPPER_LEG_LENGTH, LOWER_LEG_LENGTH, 1.0);
        assert!(
            (stretched_knee.distance(hip) + stretched_knee.distance(far_foot)
                - hip.distance(far_foot))
            .abs()
                < 1.0e-3
        );
    }

    #[test]
    fn toe_stand_raises_the_ankle_while_the_toe_stays_on_the_floor() {
        let sample = sample_phrase(&pilot_phrase(), 57.0);
        let joints = figure_joints(sample, 1.0, 0.0);
        assert!(joints.left_foot.y > 10.0);
        assert!(joints.right_foot.y > 10.0);
        assert!(joints.left_toe.y.abs() < 1.0e-4);
        assert!(joints.right_toe.y.abs() < 1.0e-4);
        assert!(joints.left_toe.x > joints.left_foot.x);
    }

    #[test]
    fn chest_pop_extends_the_torso_in_the_rendered_rig() {
        let sample = sample_phrase(&pilot_phrase(), 1.5);
        let joints = figure_joints(sample, 1.0, 0.0);
        assert!(joints.shoulder_center.distance(joints.pelvis) > TORSO_LENGTH);
    }

    #[test]
    fn head_snap_rotates_the_facial_cue_without_sliding_the_head_or_neck() {
        let neutral = figure_joints(MotionSample::NEUTRAL, 1.0, 0.0);
        let mut sample = MotionSample::NEUTRAL;
        sample.state.pose.head_snap = 0.62;
        let snapped = figure_joints(sample, 1.0, 0.0);

        assert_eq!(snapped.head_center, neutral.head_center);
        assert_eq!(snapped.neck_bottom, neutral.neck_bottom);
        assert!(snapped.head_direction.y < neutral.head_direction.y);
        assert!(snapped.head_direction.x < neutral.head_direction.x);
    }

    #[test]
    fn limb_segments_use_distinct_complementary_hues() {
        let hues = [
            LimbSegment::LeftUpperArm,
            LimbSegment::LeftLowerArm,
            LimbSegment::RightUpperArm,
            LimbSegment::RightLowerArm,
            LimbSegment::LeftUpperLeg,
            LimbSegment::LeftLowerLeg,
            LimbSegment::RightUpperLeg,
            LimbSegment::RightLowerLeg,
        ]
        .map(limb_hue);
        assert_eq!(hues, [0.0, 180.0, 90.0, 270.0, 45.0, 225.0, 135.0, 315.0]);
        for (upper, lower) in [(0, 1), (2, 3), (4, 5), (6, 7)] {
            assert_eq!((hues[upper] - hues[lower]).abs(), 180.0);
        }
    }

    #[test]
    fn torso_and_head_palette_spans_the_full_hue_cycle() {
        assert_eq!(rainbow_hue(0.0), 0.0);
        assert_eq!(rainbow_hue(0.25), 90.0);
        assert_eq!(rainbow_hue(0.5), 180.0);
        assert_eq!(rainbow_hue(0.75), 270.0);
        assert_eq!(rainbow_hue(1.0), 0.0);
    }

    #[test]
    fn dance_clock_returns_to_neutral_without_a_confident_tempo() {
        let mut clock = DanceClock::default();
        clock.advance(0.5, Some(120.0));
        assert!(clock.playing);
        assert!((clock.total_beats - 1.0).abs() < 1.0e-6);
        assert!((clock.neutral_blend - 0.5).abs() < 1.0e-6);

        clock.advance(0.5, None);
        assert!(!clock.playing);
        assert!((clock.total_beats - 1.0).abs() < 1.0e-6);
        assert_eq!(clock.neutral_blend, 1.0);
        let sample = pose_at_clock(&clock, &pilot_phrase());
        assert_eq!(sample.state.pose, MotionSample::NEUTRAL.state.pose);
        assert_eq!(sample.feet, MotionSample::NEUTRAL.feet);

        clock.advance(0.25, Some(120.0));
        assert!(clock.playing);
        assert!((clock.total_beats - 1.5).abs() < 1.0e-6);
        assert!((clock.neutral_blend - 0.75).abs() < 1.0e-6);
    }

    #[test]
    fn move_walkthrough_uses_authored_beats_then_waits_one_second() {
        let mut walkthrough = MoveWalkthrough::new(true);
        assert_eq!(walkthrough.current_move().unwrap().name, "Step-touch");

        walkthrough.advance(0.5, Some(120.0));
        assert_eq!(walkthrough.elapsed_beats, 1.0);
        assert!(walkthrough.current_move().is_some());

        walkthrough.advance(0.5, Some(120.0));
        assert_eq!(
            walkthrough.gap_remaining_secs,
            MOVE_WALKTHROUGH_GAP_TOTAL_SECS
        );
        assert!(walkthrough.current_move().is_none());
        walkthrough.advance(MOVE_WALKTHROUGH_RETURN_SECS, Some(120.0));
        assert_eq!(walkthrough.gap_remaining_secs, MOVE_WALKTHROUGH_GAP_SECS);
        assert_eq!(walkthrough.sample(), MotionSample::NEUTRAL);
        walkthrough.advance(0.6, Some(120.0));
        assert!((walkthrough.gap_remaining_secs - 0.4).abs() < 1.0e-6);
        walkthrough.advance(0.4, Some(120.0));
        assert_eq!(walkthrough.index, 1);
        assert_eq!(walkthrough.current_move().unwrap().name, "Shuffle");
        assert_eq!(walkthrough.elapsed_beats, 0.0);
    }

    #[test]
    fn move_walkthrough_returns_to_neutral_when_tempo_is_lost() {
        let mut walkthrough = MoveWalkthrough::new(true);
        walkthrough.advance(0.25, Some(120.0));
        let frozen_beats = walkthrough.elapsed_beats;
        walkthrough.advance(1.0, None);
        assert_eq!(walkthrough.elapsed_beats, frozen_beats);
        assert_eq!(walkthrough.neutral_blend, 1.0);
        assert_eq!(walkthrough.sample(), MotionSample::NEUTRAL);
    }

    #[test]
    fn disco_beams_sweep_and_blink_at_a_relaxed_rate() {
        assert_eq!(disco_beam_phase(0.0, PILOT_PHRASE_BEATS), 0.0);
        assert!((disco_beam_phase(16.0, PILOT_PHRASE_BEATS) - 0.25 * TAU).abs() < 1.0e-5);
        assert!((disco_beam_phase(PILOT_PHRASE_BEATS, PILOT_PHRASE_BEATS) - TAU).abs() < 1.0e-5);
        assert_eq!(DISCO_RAYS_PER_GROUP, 4);
        assert_eq!(disco_ray_group(0.0), 0); // left
        assert_eq!(disco_ray_group(3.9), 0);
        assert_eq!(disco_ray_group(4.0), 2); // right
        assert_eq!(disco_ray_group(7.9), 2);
        assert_eq!(disco_ray_group(8.0), 1); // center
        assert_eq!(disco_ray_group(12.0), 0);
        assert_eq!(disco_ray_group(16.0), 2);
        assert_eq!(disco_ray_alpha(0.0, 0), DISCO_RAY_MAX_ALPHA);
        assert_eq!(disco_ray_alpha(0.0, 4), DISCO_RAY_MIN_ALPHA);
        assert!(disco_ray_alpha(0.5, 0) > DISCO_RAY_MIN_ALPHA);
        assert_eq!(disco_ray_alpha(1.0, 0), DISCO_RAY_MIN_ALPHA);
        assert_eq!(disco_ray_alpha(4.0, 8), DISCO_RAY_MAX_ALPHA);
    }

    #[test]
    fn articulated_turn_foreshortens_and_reverses_the_side_view() {
        let side_on = figure_joints(MotionSample::NEUTRAL, 1.0, 0.0);
        let mut edge_on_sample = MotionSample::NEUTRAL;
        edge_on_sample.state.turn_angle = FRAC_PI_2;
        let edge_on = figure_joints(edge_on_sample, 1.0, 0.0);
        let mut opposite_side_sample = MotionSample::NEUTRAL;
        opposite_side_sample.state.turn_angle = PI;
        let opposite_side = figure_joints(opposite_side_sample, 1.0, 0.0);

        assert!(
            (side_on.left_shoulder.distance(side_on.right_shoulder) - 2.0 * SHOULDER_HALF_WIDTH)
                .abs()
                < 1.0e-4
        );
        assert!(edge_on.left_shoulder.distance(edge_on.right_shoulder) < 1.0e-4);
        assert!((edge_on.left_foot.x - edge_on.pelvis.x).abs() < 1.0e-4);
        assert!((edge_on.right_foot.x - edge_on.pelvis.x).abs() < 1.0e-4);
        assert!((edge_on.left_toe.x - edge_on.pelvis.x).abs() < 1.0e-4);
        assert!((edge_on.right_toe.x - edge_on.pelvis.x).abs() < 1.0e-4);
        assert!((edge_on.left_knee.x - edge_on.pelvis.x).abs() < 1.0e-4);
        assert!((edge_on.right_knee.x - edge_on.pelvis.x).abs() < 1.0e-4);
        assert!(opposite_side.left_shoulder.x > opposite_side.right_shoulder.x);
        assert!((opposite_side.left_foot.x - side_on.right_foot.x).abs() < 1.0e-4);
        assert!((opposite_side.right_foot.x - side_on.left_foot.x).abs() < 1.0e-4);
        assert!(
            ((opposite_side.left_toe.x - opposite_side.left_foot.x)
                + (side_on.left_toe.x - side_on.left_foot.x))
                .abs()
                < 1.0e-4
        );
        assert!(
            ((opposite_side.right_toe.x - opposite_side.right_foot.x)
                + (side_on.right_toe.x - side_on.right_foot.x))
                .abs()
                < 1.0e-4
        );
    }

    #[test]
    fn all_eight_limb_segments_have_a_trail_endpoint() {
        let endpoints = limb_endpoints(figure_joints(MotionSample::NEUTRAL, 1.0, 0.0));
        assert_eq!(endpoints.len(), 8);
    }

    #[test]
    fn limb_trails_connect_endpoints_and_expire_after_the_fade() {
        let mut trail = TrailTrack::default();
        trail.advance(Vec2::ZERO, 0.0);
        trail.advance(Vec2::new(10.0, 0.0), 0.1);
        assert_eq!(trail.segments.len(), 1);
        assert_eq!(trail.segments[0].start, Vec2::ZERO);
        assert_eq!(trail.segments[0].end, Vec2::new(10.0, 0.0));
        assert_eq!(trail_alpha(0.0), 1.0);
        let mid_fade = trail_alpha(TRAIL_LIFETIME_SECS * 0.5);
        assert!((0.0..1.0).contains(&mid_fade));
        assert_eq!(trail_alpha(TRAIL_LIFETIME_SECS), 0.0);

        trail.advance(Vec2::new(11.0, 0.0), TRAIL_LIFETIME_SECS);
        assert_eq!(trail.segments.len(), 1);
        assert_eq!(trail.segments[0].start, Vec2::new(10.0, 0.0));
        assert_eq!(trail.segments[0].end, Vec2::new(11.0, 0.0));
    }

    #[test]
    fn generated_dance_stays_inside_portrait_frame_through_multiple_loops() {
        let viewport = Vec2::new(1080.0, 1920.0);
        let floor = -viewport.y * 0.33;
        for seed in [0, 1, 7, 42, u64::MAX] {
            let phrase = generate_phrase(seed, GENERATED_PHRASE_BEATS);
            for step in 0..2048 {
                let sample = sample_phrase(&phrase, f64::from(step) * 0.25);
                let (joints, scale) =
                    fit_figure_to_viewport(figure_joints(sample, 1.8, floor), 1.8, viewport, floor);
                let limit = viewport.x * (0.5 - DANCER_FRAME_MARGIN_RATIO);
                let padding = HEAD_RADIUS * 1.1 * scale + GIZMO_LINE_WIDTH;
                for point in joints.positions() {
                    assert!(
                        point.x.abs() + padding <= limit + 0.01,
                        "seed {seed}, beat {}, x {}",
                        step as f32 * 0.25,
                        point.x
                    );
                }
            }
        }
    }

    #[test]
    fn frame_constraint_preserves_normal_pose_and_translates_feet_with_root() {
        let viewport = Vec2::new(1080.0, 1920.0);
        let original = figure_joints(MotionSample::NEUTRAL, 1.8, 0.0);
        let (centered, scale) = fit_figure_to_viewport(original, 1.8, viewport, 0.0);
        assert_eq!(centered.positions(), original.positions());
        assert_eq!(scale, 1.8);
        let mut translated = original;
        translated.map_positions(|point| point + Vec2::new(-1000.0, 0.0));
        let (fitted, scale) = fit_figure_to_viewport(translated, 1.8, viewport, 0.0);
        assert_eq!(scale, 1.8);
        assert_eq!(fitted.left_foot.y, original.left_foot.y);
        assert!(
            (fitted.left_foot.distance(fitted.pelvis)
                - original.left_foot.distance(original.pelvis))
            .abs()
                < 0.001
        );
        assert!(
            fitted
                .positions()
                .iter()
                .all(|point| point.x.abs() < viewport.x * 0.5)
        );
    }
}
