//! A collection of genuine 3D flows with shared perspective and fading trails.

mod candidates;
mod math;

use std::collections::VecDeque;

use bevy::{camera::RenderTarget, prelude::*, window::PrimaryWindow};

use super::common::{halton, simulation_delta, viewport_size};
use crate::recording::{Recording, RecordingRenderTarget};

use candidates::{CANDIDATES, Candidate};

const BASELINE_TRAJECTORIES: usize = 24;
const TRAIL_POINTS: usize = 1_350;
const TICK: f64 = 1.0 / 60.0;
const ROTATION_CYCLE_SECONDS: f64 = 60.0;

pub(super) struct AttractorsPlugin;

impl Plugin for AttractorsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(Color::srgb(0.002, 0.003, 0.009)))
            .add_systems(PostStartup, setup)
            .add_systems(Update, (preview_controls, update).chain());
    }
}

struct Trajectory {
    point: [f64; 3],
    trail: VecDeque<Vec3>,
}

impl Trajectory {
    fn advance(&mut self, candidate: &Candidate) {
        self.point = math::advance(self.point, candidate.equation, candidate.step);
        if self.trail.len() == TRAIL_POINTS {
            self.trail.pop_front();
        }
        self.trail.push_back(Vec3::new(
            self.point[0] as f32,
            self.point[1] as f32,
            self.point[2] as f32,
        ));
    }
}

#[derive(Resource)]
struct Attractor {
    trajectories: Vec<Trajectory>,
    elapsed: f64,
    pending: f64,
    candidate_index: usize,
    just_reset: bool,
    framing_radius: f32,
}

impl Attractor {
    fn new(candidate_index: usize) -> Self {
        let candidate = &CANDIDATES[candidate_index];
        Self {
            trajectories: (1..=candidate.default_trajectories)
                .map(|index| create_trajectory(index, candidate))
                .collect(),
            elapsed: 0.0,
            pending: 0.0,
            candidate_index,
            just_reset: true,
            framing_radius: candidate.framing_radius,
        }
    }
}

#[derive(Component)]
struct AttractorCamera;

#[derive(Component)]
struct PreviewReadout;

fn candidate_from_args() -> usize {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--candidate" {
            return args
                .next()
                .and_then(|value| value.parse::<usize>().ok())
                .and_then(|number| {
                    CANDIDATES
                        .iter()
                        .position(|candidate| candidate.id == number)
                })
                .unwrap_or_else(|| {
                    eprintln!(
                        "--candidate requires an available candidate ID: {}",
                        CANDIDATES
                            .iter()
                            .map(|candidate| candidate.id.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                    std::process::exit(2);
                });
        }
    }
    0
}

/// Numerical preflight without a window, GPU, microphone, or frame output.
pub(super) fn validate_candidates() -> bool {
    let mut all_valid = true;
    for candidate in CANDIDATES {
        let mut max_radius = 0.0_f64;
        let mut max_error = 0.0_f64;
        let mut extent = 0.0_f64;
        let mut valid = true;
        for seed in 1..=BASELINE_TRAJECTORIES.max(candidate.default_trajectories) {
            let trajectory = create_trajectory(seed, candidate);
            let mut point = trajectory.point;
            let mut min = [f64::INFINITY; 3];
            let mut max = [f64::NEG_INFINITY; 3];
            for p in &trajectory.trail {
                if !p.is_finite() {
                    valid = false;
                    break;
                }
                let centered = *p - Vec3::from_array(candidate.view_center);
                max_radius = max_radius.max(f64::from(centered.length()));
            }
            let mut fine = point;
            let mut coarse = point;
            for _ in 0..20 {
                coarse = math::advance(coarse, candidate.equation, candidate.step);
                fine = math::advance(fine, candidate.equation, candidate.step * 0.5);
                fine = math::advance(fine, candidate.equation, candidate.step * 0.5);
            }
            for axis in 0..3 {
                max_error = max_error.max((coarse[axis] - fine[axis]).abs());
            }
            // Four minutes of playback, including the intended 60-second reel.
            for _ in 0..14_400 * candidate.steps_per_tick {
                point = math::advance(point, candidate.equation, candidate.step);
                if point
                    .iter()
                    .any(|value| !value.is_finite() || value.abs() > 1e6)
                {
                    valid = false;
                    break;
                }
                let radius = point
                    .iter()
                    .zip(candidate.view_center)
                    .map(|(value, center)| (value - f64::from(center)).powi(2))
                    .sum::<f64>()
                    .sqrt();
                max_radius = max_radius.max(radius);
                for axis in 0..3 {
                    min[axis] = min[axis].min(point[axis]);
                    max[axis] = max[axis].max(point[axis]);
                }
            }
            extent = extent.max(
                (0..3)
                    .map(|axis| (max[axis] - min[axis]).powi(2))
                    .sum::<f64>()
                    .sqrt(),
            );
        }
        valid &=
            max_radius <= f64::from(candidate.framing_radius) && max_error < 0.001 && extent > 0.1;
        all_valid &= valid;
        println!(
            "CANDIDATE {:02} {} {} radius={max_radius:.3}/{} rk4_error={max_error:.8} extent={extent:.3} {}",
            candidate.id,
            candidate.name,
            candidate.equation.parameters(),
            candidate.framing_radius,
            if valid { "PASS" } else { "FAIL" }
        );
    }
    all_valid
}

fn setup(
    mut commands: Commands,
    render_target: Option<Res<RecordingRenderTarget>>,
    mut configs: ResMut<GizmoConfigStore>,
    recording: Option<Res<Recording>>,
) {
    let mut camera = commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 40.0).looking_at(Vec3::ZERO, Vec3::Y),
        AttractorCamera,
    ));
    if let Some(target) = render_target {
        camera.insert(RenderTarget::Image(target.0.clone().into()));
    }
    let (config, _) = configs.config_mut::<DefaultGizmoConfigGroup>();
    config.line.width = 2.0;
    let selected = candidate_from_args();
    commands.insert_resource(Attractor::new(selected));
    if recording.is_none() {
        commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(16.0),
                left: Val::Px(16.0),
                ..default()
            },
            Text::new(""),
            TextFont {
                font_size: FontSize::Px(18.0),
                ..default()
            },
            TextColor(Color::srgb(0.85, 0.9, 1.0)),
            PreviewReadout,
        ));
    }
    info!(
        "Attractor {:02}: {} | {}",
        CANDIDATES[selected].id,
        CANDIDATES[selected].name,
        CANDIDATES[selected].equation.parameters()
    );
}

fn create_trajectory(index: usize, candidate: &Candidate) -> Trajectory {
    let index = index as u32;
    let mut point = [
        f64::from(halton(index, 2) * 2.0 - 1.0),
        f64::from(halton(index, 3) * 2.0 - 1.0),
        f64::from(halton(index, 5) * 2.0 - 1.0),
    ];
    for (value, center) in point.iter_mut().zip(candidate.initial_center) {
        *value = center + *value * candidate.initial_extent;
    }
    for _ in 0..candidate.warmup_steps {
        point = math::advance(point, candidate.equation, candidate.step);
    }
    let mut trajectory = Trajectory {
        point,
        trail: VecDeque::with_capacity(TRAIL_POINTS),
    };
    for _ in 0..TRAIL_POINTS {
        trajectory.advance(candidate);
    }
    trajectory
}

fn preview_controls(
    keys: Res<ButtonInput<KeyCode>>,
    recording: Option<Res<Recording>>,
    attractor: Option<ResMut<Attractor>>,
    window: Option<Single<&mut Window, With<PrimaryWindow>>>,
    mut readout: Query<&mut Text, With<PreviewReadout>>,
) {
    if recording.is_some() {
        return;
    }
    let Some(mut attractor) = attractor else {
        return;
    };
    let mut selected = attractor.candidate_index;
    if keys.just_pressed(KeyCode::ArrowRight) {
        selected = (selected + 1) % CANDIDATES.len();
    } else if keys.just_pressed(KeyCode::ArrowLeft) {
        selected = (selected + CANDIDATES.len() - 1) % CANDIDATES.len();
    }
    let page = if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
        9
    } else {
        0
    };
    for (index, key) in [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
    ]
    .into_iter()
    .enumerate()
    {
        let id = index + page + 1;
        if keys.just_pressed(key)
            && let Some(index) = CANDIDATES.iter().position(|candidate| candidate.id == id)
        {
            selected = index;
        }
    }
    let restart = keys.just_pressed(KeyCode::KeyR);
    if selected != attractor.candidate_index || restart {
        *attractor = Attractor::new(selected);
        info!(
            "Restarted candidate {:02}: {} | {}",
            CANDIDATES[selected].id,
            CANDIDATES[selected].name,
            CANDIDATES[selected].equation.parameters()
        );
    }
    let candidate = &CANDIDATES[attractor.candidate_index];
    let current = attractor.trajectories.len();
    let count = if keys.just_pressed(KeyCode::ArrowUp) {
        current.saturating_add((current / 2).max(1))
    } else if keys.just_pressed(KeyCode::ArrowDown) {
        (current * 2 / 3).max(1)
    } else if keys.just_pressed(KeyCode::Backspace) {
        candidate.default_trajectories
    } else {
        current
    };
    if count != current {
        if count < current {
            attractor.trajectories.truncate(count);
        } else {
            attractor
                .trajectories
                .extend((current + 1..=count).map(|index| create_trajectory(index, candidate)));
        }
        info!(
            "Density: {count} trajectories × {TRAIL_POINTS} points; candidate default is {}",
            candidate.default_trajectories
        );
    }
    let label = format!(
        "{:02} {} | {} | {count} × {TRAIL_POINTS} | {} candidates",
        candidate.id,
        candidate.name,
        candidate.equation.parameters(),
        CANDIDATES.len()
    );
    if let Some(mut window) = window
        && window.title != label
    {
        window.title = label.clone();
    }
    let text = format!(
        "{label}\n←/→ candidate · 1–9 / Shift+1–2 select · R restart\n↑/↓ density · Backspace default"
    );
    for mut readout in &mut readout {
        if readout.0 != text {
            readout.0 = text.clone();
        }
    }
}

fn update(
    time: Res<Time>,
    recording: Option<Res<Recording>>,
    window: Option<Single<&Window, With<PrimaryWindow>>>,
    attractor: Option<ResMut<Attractor>>,
    mut camera: Query<(&mut Transform, &Projection), With<AttractorCamera>>,
    mut gizmos: Gizmos,
) {
    let Some(mut attractor) = attractor else {
        return;
    };
    let candidate = &CANDIDATES[attractor.candidate_index];
    let dt = simulation_delta(recording.as_deref(), time.delta_secs());
    if attractor.just_reset {
        attractor.just_reset = false;
    } else {
        attractor.pending += f64::from(if recording.is_some() { dt } else { dt.min(0.1) });
    }
    while attractor.pending + 1e-9 >= TICK {
        attractor.pending -= TICK;
        attractor.elapsed += TICK;
        for trajectory in &mut attractor.trajectories {
            for _ in 0..candidate.steps_per_tick {
                trajectory.advance(candidate);
            }
        }
    }
    // Expand framing if a future orbit or extra density seed exceeds its sampled bounds.
    let observed_radius = attractor
        .trajectories
        .iter()
        .flat_map(|trajectory| &trajectory.trail)
        .map(|point| (*point - Vec3::from_array(candidate.view_center)).length())
        .fold(0.0_f32, f32::max);
    if observed_radius > attractor.framing_radius {
        attractor.framing_radius = observed_radius * 1.08;
    }
    let window_size = window.as_ref().map(|w| Vec2::new(w.width(), w.height()));
    if let Some(viewport) = viewport_size(recording.as_deref(), window_size) {
        for (mut transform, projection) in &mut camera {
            if let Projection::Perspective(perspective) = projection {
                // Candidate bounds encompass its geometry through the complete rotation.
                let radius = attractor.framing_radius;
                let half_angle =
                    ((perspective.fov * 0.5).tan() * (viewport.x / viewport.y).min(1.0)).atan();
                let distance = radius / half_angle.sin() * 1.08;
                *transform =
                    Transform::from_xyz(0.0, 0.0, distance).looking_at(Vec3::ZERO, Vec3::Y);
            }
        }
    }
    let elapsed = attractor.elapsed as f32;
    // Fixed composition order: local X, then Y, then Z. All angles reset after 60 s.
    let phase = (attractor.elapsed.rem_euclid(ROTATION_CYCLE_SECONDS) / ROTATION_CYCLE_SECONDS
        * std::f64::consts::TAU) as f32;
    let rotation = Quat::from_rotation_z(phase * 4.0)
        * Quat::from_rotation_y(phase * 2.0)
        * Quat::from_rotation_x(phase);
    for (index, trajectory) in attractor.trajectories.iter().enumerate() {
        let mut previous = None;
        for (age, point) in trajectory.trail.iter().enumerate() {
            let point = rotation * (*point - Vec3::from_array(candidate.view_center));
            if let Some(start) = previous {
                let fraction = age as f32 / (TRAIL_POINTS - 1) as f32;
                let hue = (index as f32 * (360.0 / BASELINE_TRAJECTORIES as f32)
                    + fraction * 90.0
                    + elapsed * 8.0)
                    % 360.0;
                let alpha = 0.04 + fraction.powi(2) * 0.85;
                gizmos.line(start, point, Color::hsla(hue, 0.95, 0.65, alpha));
            }
            previous = Some(point);
        }
    }
}
