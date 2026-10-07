//! Bevy-independent dance vocabulary and layered, deterministic beat sampling.

use std::f32::consts::{PI, TAU};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Vec2 {
    pub(crate) x: f32,
    pub(crate) y: f32,
}

impl Vec2 {
    pub(crate) const ZERO: Self = Self { x: 0.0, y: 0.0 };

    pub(crate) const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    fn lerp(self, target: Self, t: f32) -> Self {
        Self::new(lerp(self.x, target.x, t), lerp(self.y, target.y, t))
    }

    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y)
    }

    fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y)
    }

    fn scale(self, amount: f32) -> Self {
        Self::new(self.x * amount, self.y * amount)
    }
}

const LEFT_HOME: Vec2 = Vec2::new(-22.0, 0.0);
const RIGHT_HOME: Vec2 = Vec2::new(22.0, 0.0);
const HOME_FEET: [Vec2; 2] = [LEFT_HOME, RIGHT_HOME];
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DancePose {
    pub(crate) torso_lean: f32,
    pub(crate) squat: f32,
    pub(crate) pelvis_lift: f32,
    /// Whole-body rigid lean about the ankles (MJ-style Smooth Criminal lean).
    pub(crate) ankle_lean: f32,
    pub(crate) left_shoulder: f32,
    pub(crate) right_shoulder: f32,
    pub(crate) left_elbow: f32,
    pub(crate) right_elbow: f32,
    pub(crate) chest_pop: f32,
    pub(crate) left_shoulder_lift: f32,
    pub(crate) right_shoulder_lift: f32,
    /// Head rotation in radians, used by the rendered facial cue.
    pub(crate) head_snap: f32,
    pub(crate) left_toe_pitch: f32,
    pub(crate) right_toe_pitch: f32,
}

pub(crate) const NEUTRAL_POSE: DancePose = DancePose {
    torso_lean: 0.0,
    squat: 0.0,
    pelvis_lift: 0.0,
    ankle_lean: 0.0,
    left_shoulder: 0.38,
    right_shoulder: 0.38,
    left_elbow: 0.25,
    right_elbow: 0.25,
    chest_pop: 0.0,
    left_shoulder_lift: 0.0,
    right_shoulder_lift: 0.0,
    head_snap: 0.0,
    left_toe_pitch: 0.0,
    right_toe_pitch: 0.0,
};

/// Additive controls layered over the continuous groove and footwork pose.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct PoseDelta {
    pub(crate) torso_lean: f32,
    pub(crate) squat: f32,
    pub(crate) pelvis_lift: f32,
    pub(crate) ankle_lean: f32,
    pub(crate) left_shoulder: f32,
    pub(crate) right_shoulder: f32,
    pub(crate) left_elbow: f32,
    pub(crate) right_elbow: f32,
    pub(crate) chest_pop: f32,
    pub(crate) left_shoulder_lift: f32,
    pub(crate) right_shoulder_lift: f32,
    /// Head rotation in radians, used by the rendered facial cue.
    pub(crate) head_snap: f32,
    pub(crate) left_toe_pitch: f32,
    pub(crate) right_toe_pitch: f32,
}

impl PoseDelta {
    const ZERO: Self = Self {
        torso_lean: 0.0,
        squat: 0.0,
        pelvis_lift: 0.0,
        ankle_lean: 0.0,
        left_shoulder: 0.0,
        right_shoulder: 0.0,
        left_elbow: 0.0,
        right_elbow: 0.0,
        chest_pop: 0.0,
        left_shoulder_lift: 0.0,
        right_shoulder_lift: 0.0,
        head_snap: 0.0,
        left_toe_pitch: 0.0,
        right_toe_pitch: 0.0,
    };

    fn add_to(self, pose: &mut DancePose, weight: f32) {
        pose.torso_lean += self.torso_lean * weight;
        pose.squat += self.squat * weight;
        pose.pelvis_lift += self.pelvis_lift * weight;
        pose.ankle_lean += self.ankle_lean * weight;
        pose.left_shoulder += self.left_shoulder * weight;
        pose.right_shoulder += self.right_shoulder * weight;
        pose.left_elbow += self.left_elbow * weight;
        pose.right_elbow += self.right_elbow * weight;
        pose.chest_pop += self.chest_pop * weight;
        pose.left_shoulder_lift += self.left_shoulder_lift * weight;
        pose.right_shoulder_lift += self.right_shoulder_lift * weight;
        pose.head_snap += self.head_snap * weight;
        pose.left_toe_pitch += self.left_toe_pitch * weight;
        pose.right_toe_pitch += self.right_toe_pitch * weight;
    }

    fn mirrored(self) -> Self {
        Self {
            torso_lean: -self.torso_lean,
            ankle_lean: -self.ankle_lean,
            left_shoulder: self.right_shoulder,
            right_shoulder: self.left_shoulder,
            left_elbow: self.right_elbow,
            right_elbow: self.left_elbow,
            left_shoulder_lift: self.right_shoulder_lift,
            right_shoulder_lift: self.left_shoulder_lift,
            left_toe_pitch: self.right_toe_pitch,
            right_toe_pitch: self.left_toe_pitch,
            head_snap: -self.head_snap,
            ..self
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MotionState {
    pub(crate) pose: DancePose,
    pub(crate) root: Vec2,
    /// Unwrapped yaw angle projected into side-view foreshortening.
    pub(crate) turn_angle: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum FootPart {
    Flat,
    Heel,
    Ball,
    Toe,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum FootIntent {
    /// A stage-relative anchor, held fixed until another intent moves the foot.
    Plant {
        anchor: Vec2,
        part: FootPart,
        weight: f32,
    },
    /// An airborne foot target, with vertical lift above the target path.
    Swing { target: Vec2, lift: f32 },
    /// A floor-contact target that glides rather than lifting.
    Slide {
        target: Vec2,
        part: FootPart,
        weight: f32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FootContact {
    Planted,
    Swinging,
    Sliding,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FootSample {
    /// Phrase-relative ankle position; the renderer adds floor height and toe geometry.
    pub(crate) position: Vec2,
    pub(crate) lift: f32,
    pub(crate) contact: FootContact,
    /// Which part of the foot carries contact with the floor.
    pub(crate) part: FootPart,
    /// Fraction of body weight borne by this foot (0..1).
    pub(crate) weight: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MotionSample {
    pub(crate) state: MotionState,
    pub(crate) feet: [FootSample; 2],
}

impl MotionSample {
    pub(crate) const NEUTRAL: Self = Self {
        state: MotionState {
            pose: NEUTRAL_POSE,
            root: Vec2::ZERO,
            turn_angle: 0.0,
        },
        feet: [
            FootSample {
                position: LEFT_HOME,
                lift: 0.0,
                contact: FootContact::Planted,
                part: FootPart::Flat,
                weight: 0.5,
            },
            FootSample {
                position: RIGHT_HOME,
                lift: 0.0,
                contact: FootContact::Planted,
                part: FootPart::Flat,
                weight: 0.5,
            },
        ],
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ActionId {
    StepTouch,
    Shuffle,
    Stomp,
    HeelToe,
    CrossStep,
    Backstep,
    KneeLift,
    KickStep,
    Jump,
    Slide,
    Pivot,
    Moonwalk,
    ChestPop,
    ShoulderHit,
    HeadSnap,
    Freeze,
    Crouch,
    Lean,
    ToeStand,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ActionLayer {
    Footwork,
    Accent,
    Pose,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ActionTag {
    Grounded,
    Traveling,
    Accent,
    Isolating,
    Airborne,
    Balance,
    Held,
    Signature,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Side {
    Right,
    Left,
    Both,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Easing {
    Linear,
    EaseInOut,
    EaseIn,
    EaseOut,
    Hold,
}

impl Easing {
    fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => t,
            Self::EaseInOut => t * t * (3.0 - 2.0 * t),
            Self::EaseIn => t * t,
            Self::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
            Self::Hold if t < 1.0 => 0.0,
            Self::Hold => 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ActionKeyframe {
    phase: f32,
    body: PoseDelta,
    root: Vec2,
    turn_angle: f32,
    feet: [FootIntent; 2],
    easing: Easing,
}

#[derive(Clone, Copy, Debug)]
enum ActionShape {
    Footwork(&'static [ActionKeyframe]),
    Pulse(PoseDelta),
    Hold(PoseDelta),
}

#[derive(Clone, Copy, Debug)]
struct ActionDefinition {
    id: ActionId,
    layer: ActionLayer,
    tags: &'static [ActionTag],
    shape: ActionShape,
    /// A freeze mutes only the continuous groove layer; its authored pose remains active.
    freezes_groove: bool,
    /// Phase at which this move is considered finished for procedural chaining.
    /// The remainder of the keyframes return toward neutral and are only played
    /// in walkthrough/demo mode.
    exit_phase: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ActionInstance {
    id: ActionId,
    start_beat: f32,
    beats: f32,
    side: Side,
    intensity: f32,
    travel: f32,
    /// World-space foot anchors where this move begins; used to chain moves
    /// without returning to neutral.
    entry_feet: [Vec2; 2],
    /// Additional turn accumulated before this move begins.
    entry_turn: f32,
}

#[derive(Clone, Debug)]
pub(crate) struct Phrase {
    generated: bool,
    pub(crate) beats: f32,
    pub(crate) events: Vec<ActionInstance>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct WalkthroughMove {
    pub(crate) id: ActionId,
    pub(crate) name: &'static str,
    pub(crate) beats: f32,
    side: Side,
}

pub(crate) const PILOT_PHRASE_BEATS: f32 = 64.0;

const fn body(
    torso_lean: f32,
    squat: f32,
    pelvis_lift: f32,
    left_shoulder: f32,
    right_shoulder: f32,
    left_elbow: f32,
    right_elbow: f32,
) -> PoseDelta {
    PoseDelta {
        torso_lean,
        squat,
        pelvis_lift,
        left_shoulder,
        right_shoulder,
        left_elbow,
        right_elbow,
        ..PoseDelta::ZERO
    }
}

const fn frame(
    phase: f32,
    body: PoseDelta,
    root: Vec2,
    turn_angle: f32,
    left: FootIntent,
    right: FootIntent,
    easing: Easing,
) -> ActionKeyframe {
    ActionKeyframe {
        phase,
        body,
        root,
        turn_angle,
        feet: [left, right],
        easing,
    }
}

const fn plant(anchor: Vec2) -> FootIntent {
    FootIntent::Plant {
        anchor,
        part: FootPart::Flat,
        weight: 0.5,
    }
}

const fn plant_weighted(anchor: Vec2, part: FootPart, weight: f32) -> FootIntent {
    FootIntent::Plant {
        anchor,
        part,
        weight,
    }
}

const fn swing(target: Vec2, lift: f32) -> FootIntent {
    FootIntent::Swing { target, lift }
}

const fn slide(target: Vec2) -> FootIntent {
    FootIntent::Slide {
        target,
        part: FootPart::Flat,
        weight: 0.5,
    }
}

const fn slide_weighted(target: Vec2, part: FootPart, weight: f32) -> FootIntent {
    FootIntent::Slide {
        target,
        part,
        weight,
    }
}

const fn event(id: ActionId, start_beat: f32, beats: f32, side: Side) -> ActionInstance {
    ActionInstance {
        id,
        start_beat,
        beats,
        side,
        intensity: 1.0,
        travel: 1.0,
        entry_feet: HOME_FEET,
        entry_turn: 0.0,
    }
}

const fn footwork(
    id: ActionId,
    keyframes: &'static [ActionKeyframe],
    tags: &'static [ActionTag],
    exit_phase: f32,
) -> ActionDefinition {
    ActionDefinition {
        id,
        layer: ActionLayer::Footwork,
        tags,
        shape: ActionShape::Footwork(keyframes),
        freezes_groove: false,
        exit_phase,
    }
}

const fn pulse(id: ActionId, target: PoseDelta, tags: &'static [ActionTag]) -> ActionDefinition {
    ActionDefinition {
        id,
        layer: ActionLayer::Accent,
        tags,
        shape: ActionShape::Pulse(target),
        freezes_groove: false,
        exit_phase: 1.0,
    }
}

const fn pose(
    id: ActionId,
    target: PoseDelta,
    tags: &'static [ActionTag],
    freezes_groove: bool,
) -> ActionDefinition {
    ActionDefinition {
        id,
        layer: ActionLayer::Pose,
        tags,
        shape: ActionShape::Hold(target),
        freezes_groove,
        exit_phase: 1.0,
    }
}

const STEP_TOUCH: [ActionKeyframe; 5] = [
    frame(
        0.0,
        PoseDelta::ZERO,
        Vec2::ZERO,
        0.0,
        plant_weighted(LEFT_HOME, FootPart::Flat, 0.5),
        plant_weighted(RIGHT_HOME, FootPart::Flat, 0.5),
        Easing::EaseInOut,
    ),
    frame(
        0.25,
        body(-0.1, 6.0, 0.0, 0.08, -0.04, 0.0, 0.0),
        Vec2::new(4.0, 0.0),
        0.0,
        swing(Vec2::new(-10.0, 0.0), 4.0),
        plant_weighted(Vec2::new(34.0, 0.0), FootPart::Flat, 1.0),
        Easing::EaseOut,
    ),
    frame(
        0.5,
        body(-0.08, 4.0, 0.0, 0.06, -0.02, 0.0, 0.0),
        Vec2::new(6.0, 0.0),
        0.0,
        plant_weighted(Vec2::new(-6.0, 0.0), FootPart::Toe, 0.0),
        plant_weighted(Vec2::new(34.0, 0.0), FootPart::Flat, 1.0),
        Easing::Hold,
    ),
    frame(
        0.75,
        body(0.08, 6.0, 0.0, -0.04, 0.08, 0.0, 0.0),
        Vec2::new(-4.0, 0.0),
        0.0,
        plant_weighted(Vec2::new(-34.0, 0.0), FootPart::Flat, 1.0),
        swing(Vec2::new(10.0, 0.0), 4.0),
        Easing::EaseInOut,
    ),
    frame(
        1.0,
        PoseDelta::ZERO,
        Vec2::ZERO,
        0.0,
        plant_weighted(LEFT_HOME, FootPart::Flat, 0.5),
        plant_weighted(RIGHT_HOME, FootPart::Flat, 0.5),
        Easing::Linear,
    ),
];

const HEEL_TOE: [ActionKeyframe; 5] = [
    frame(
        0.0,
        PoseDelta::ZERO,
        Vec2::ZERO,
        0.0,
        plant_weighted(LEFT_HOME, FootPart::Flat, 0.5),
        plant_weighted(RIGHT_HOME, FootPart::Flat, 0.5),
        Easing::EaseInOut,
    ),
    frame(
        0.25,
        body(-0.06, 4.0, 0.0, 0.04, 0.0, 0.0, 0.0),
        Vec2::ZERO,
        0.0,
        plant_weighted(LEFT_HOME, FootPart::Flat, 0.8),
        plant_weighted(RIGHT_HOME, FootPart::Heel, 0.2),
        Easing::EaseOut,
    ),
    frame(
        0.5,
        body(-0.04, 3.0, 0.0, 0.02, 0.0, 0.0, 0.0),
        Vec2::ZERO,
        0.0,
        plant_weighted(LEFT_HOME, FootPart::Flat, 0.3),
        plant_weighted(RIGHT_HOME, FootPart::Ball, 0.7),
        Easing::EaseInOut,
    ),
    frame(
        0.75,
        body(-0.02, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0),
        Vec2::ZERO,
        0.0,
        plant_weighted(LEFT_HOME, FootPart::Flat, 0.1),
        plant_weighted(RIGHT_HOME, FootPart::Toe, 0.9),
        Easing::EaseInOut,
    ),
    frame(
        1.0,
        PoseDelta::ZERO,
        Vec2::ZERO,
        0.0,
        plant_weighted(LEFT_HOME, FootPart::Flat, 0.5),
        plant_weighted(RIGHT_HOME, FootPart::Flat, 0.5),
        Easing::Linear,
    ),
];

const CROSS_STEP: [ActionKeyframe; 5] = [
    frame(
        0.0,
        PoseDelta::ZERO,
        Vec2::ZERO,
        0.0,
        plant(LEFT_HOME),
        plant(RIGHT_HOME),
        Easing::EaseInOut,
    ),
    frame(
        0.22,
        body(-0.22, 12.0, 0.0, 0.18, -0.1, 0.0, 0.0),
        Vec2::new(-5.0, 0.0),
        0.0,
        plant(LEFT_HOME),
        swing(Vec2::new(-8.0, 0.0), 12.0),
        Easing::EaseOut,
    ),
    frame(
        0.5,
        body(-0.25, 8.0, 0.0, 0.16, -0.08, 0.0, 0.0),
        Vec2::new(-8.0, 0.0),
        0.0,
        plant(LEFT_HOME),
        plant(Vec2::new(-10.0, 0.0)),
        Easing::Hold,
    ),
    frame(
        0.78,
        body(0.1, 6.0, 0.0, -0.04, 0.08, 0.0, 0.0),
        Vec2::new(-3.0, 0.0),
        0.0,
        plant(LEFT_HOME),
        swing(RIGHT_HOME, 8.0),
        Easing::EaseInOut,
    ),
    frame(
        1.0,
        PoseDelta::ZERO,
        Vec2::ZERO,
        0.0,
        plant(LEFT_HOME),
        plant(RIGHT_HOME),
        Easing::Linear,
    ),
];

const BACKSTEP: [ActionKeyframe; 4] = [
    frame(
        0.0,
        PoseDelta::ZERO,
        Vec2::ZERO,
        0.0,
        plant(LEFT_HOME),
        plant(RIGHT_HOME),
        Easing::EaseInOut,
    ),
    frame(
        0.28,
        body(-0.22, 14.0, 0.0, 0.12, -0.08, 0.0, 0.0),
        Vec2::ZERO,
        0.0,
        plant(LEFT_HOME),
        swing(Vec2::new(7.0, 0.0), 10.0),
        Easing::EaseOut,
    ),
    frame(
        0.68,
        body(-0.18, 8.0, 0.0, 0.1, -0.06, 0.0, 0.0),
        Vec2::ZERO,
        0.0,
        plant(LEFT_HOME),
        plant(Vec2::new(7.0, 0.0)),
        Easing::EaseInOut,
    ),
    frame(
        1.0,
        PoseDelta::ZERO,
        Vec2::ZERO,
        0.0,
        plant(LEFT_HOME),
        plant(RIGHT_HOME),
        Easing::Linear,
    ),
];

const KNEE_LIFT: [ActionKeyframe; 5] = [
    frame(
        0.0,
        PoseDelta::ZERO,
        Vec2::ZERO,
        0.0,
        plant(LEFT_HOME),
        plant(RIGHT_HOME),
        Easing::EaseInOut,
    ),
    frame(
        0.24,
        body(-0.18, 12.0, 0.0, -0.12, 0.12, 0.0, 0.0),
        Vec2::new(-3.0, 0.0),
        0.0,
        plant(LEFT_HOME),
        swing(RIGHT_HOME, 22.0),
        Easing::EaseOut,
    ),
    frame(
        0.5,
        body(-0.3, 6.0, 0.0, -0.18, 0.2, 0.12, 0.0),
        Vec2::new(-5.0, 0.0),
        0.0,
        plant(LEFT_HOME),
        swing(Vec2::new(28.0, 0.0), 58.0),
        Easing::Hold,
    ),
    frame(
        0.78,
        body(-0.16, 10.0, 0.0, -0.1, 0.12, 0.0, 0.0),
        Vec2::new(-2.0, 0.0),
        0.0,
        plant(LEFT_HOME),
        swing(RIGHT_HOME, 20.0),
        Easing::EaseIn,
    ),
    frame(
        1.0,
        PoseDelta::ZERO,
        Vec2::ZERO,
        0.0,
        plant(LEFT_HOME),
        plant(RIGHT_HOME),
        Easing::Linear,
    ),
];

const KICK_STEP: [ActionKeyframe; 5] = [
    frame(
        0.0,
        PoseDelta::ZERO,
        Vec2::ZERO,
        0.0,
        plant(LEFT_HOME),
        plant(RIGHT_HOME),
        Easing::EaseIn,
    ),
    frame(
        0.2,
        body(-0.28, 20.0, 0.0, -0.15, 0.3, 0.0, 0.0),
        Vec2::new(-6.0, 0.0),
        0.0,
        plant(LEFT_HOME),
        swing(RIGHT_HOME, 18.0),
        Easing::EaseOut,
    ),
    frame(
        0.46,
        body(-0.38, 6.0, 0.0, -0.12, 0.4, 0.08, 0.0),
        Vec2::new(-12.0, 0.0),
        0.0,
        plant(LEFT_HOME),
        swing(Vec2::new(76.0, 0.0), 52.0),
        Easing::EaseOut,
    ),
    frame(
        0.72,
        body(-0.2, 10.0, 0.0, 0.0, 0.18, 0.0, 0.0),
        Vec2::new(-6.0, 0.0),
        0.0,
        plant(LEFT_HOME),
        swing(RIGHT_HOME, 10.0),
        Easing::EaseIn,
    ),
    frame(
        1.0,
        PoseDelta::ZERO,
        Vec2::ZERO,
        0.0,
        plant(LEFT_HOME),
        plant(RIGHT_HOME),
        Easing::Linear,
    ),
];

const STOMP: [ActionKeyframe; 4] = [
    frame(
        0.0,
        PoseDelta::ZERO,
        Vec2::ZERO,
        0.0,
        plant(LEFT_HOME),
        plant(RIGHT_HOME),
        Easing::EaseIn,
    ),
    frame(
        0.3,
        body(0.12, 26.0, 0.0, 0.3, 0.5, 0.0, 0.0),
        Vec2::new(0.0, 0.0),
        0.0,
        plant(LEFT_HOME),
        swing(RIGHT_HOME, 30.0),
        Easing::EaseOut,
    ),
    frame(
        0.68,
        body(-0.16, 20.0, 0.0, 0.0, 0.4, 0.0, 0.0),
        Vec2::ZERO,
        0.0,
        plant(LEFT_HOME),
        plant(RIGHT_HOME),
        Easing::EaseIn,
    ),
    frame(
        1.0,
        PoseDelta::ZERO,
        Vec2::ZERO,
        0.0,
        plant(LEFT_HOME),
        plant(RIGHT_HOME),
        Easing::Linear,
    ),
];

const JUMP: [ActionKeyframe; 5] = [
    frame(
        0.0,
        PoseDelta::ZERO,
        Vec2::ZERO,
        0.0,
        plant(LEFT_HOME),
        plant(RIGHT_HOME),
        Easing::EaseIn,
    ),
    frame(
        0.2,
        body(-0.18, 48.0, 0.0, -0.18, -0.18, 0.0, 0.0),
        Vec2::ZERO,
        0.0,
        plant(LEFT_HOME),
        plant(RIGHT_HOME),
        Easing::EaseOut,
    ),
    frame(
        0.45,
        body(0.08, 0.0, 28.0, 1.6, 1.6, 0.3, 0.3),
        Vec2::ZERO,
        0.0,
        swing(LEFT_HOME, 30.0),
        swing(RIGHT_HOME, 30.0),
        Easing::EaseIn,
    ),
    frame(
        0.68,
        body(0.0, 0.0, 74.0, 1.9, 1.9, 0.45, 0.45),
        Vec2::ZERO,
        0.0,
        swing(LEFT_HOME, 78.0),
        swing(RIGHT_HOME, 78.0),
        Easing::EaseOut,
    ),
    frame(
        1.0,
        PoseDelta::ZERO,
        Vec2::ZERO,
        0.0,
        plant(LEFT_HOME),
        plant(RIGHT_HOME),
        Easing::Linear,
    ),
];

const SHUFFLE: [ActionKeyframe; 7] = [
    frame(
        0.0,
        PoseDelta::ZERO,
        Vec2::ZERO,
        0.0,
        plant(LEFT_HOME),
        plant(RIGHT_HOME),
        Easing::EaseInOut,
    ),
    frame(
        0.16,
        body(-0.12, 12.0, 0.0, 0.1, -0.04, 0.0, 0.0),
        Vec2::ZERO,
        0.0,
        plant(LEFT_HOME),
        swing(Vec2::new(34.0, 0.0), 8.0),
        Easing::EaseOut,
    ),
    frame(
        0.32,
        body(-0.16, 8.0, 0.0, 0.12, -0.06, 0.0, 0.0),
        Vec2::new(4.0, 0.0),
        0.0,
        slide(Vec2::new(-30.0, 0.0)),
        plant(Vec2::new(34.0, 0.0)),
        Easing::EaseInOut,
    ),
    frame(
        0.5,
        body(0.12, 10.0, 0.0, -0.04, 0.1, 0.0, 0.0),
        Vec2::ZERO,
        0.0,
        plant(LEFT_HOME),
        plant(RIGHT_HOME),
        Easing::EaseInOut,
    ),
    frame(
        0.66,
        body(0.16, 8.0, 0.0, -0.06, 0.12, 0.0, 0.0),
        Vec2::new(-4.0, 0.0),
        0.0,
        swing(Vec2::new(-34.0, 0.0), 8.0),
        plant(RIGHT_HOME),
        Easing::EaseOut,
    ),
    frame(
        0.84,
        body(0.12, 6.0, 0.0, -0.04, 0.1, 0.0, 0.0),
        Vec2::ZERO,
        0.0,
        plant(LEFT_HOME),
        slide(Vec2::new(30.0, 0.0)),
        Easing::EaseInOut,
    ),
    frame(
        1.0,
        PoseDelta::ZERO,
        Vec2::ZERO,
        0.0,
        plant(LEFT_HOME),
        plant(RIGHT_HOME),
        Easing::Linear,
    ),
];

const SLIDE: [ActionKeyframe; 5] = [
    frame(
        0.0,
        body(0.0, 10.0, 0.0, 0.12, -0.08, 0.0, 0.0),
        Vec2::ZERO,
        0.0,
        plant_weighted(LEFT_HOME, FootPart::Flat, 0.5),
        plant_weighted(RIGHT_HOME, FootPart::Flat, 0.5),
        Easing::EaseIn,
    ),
    frame(
        0.25,
        body(-0.26, 16.0, 0.0, 0.2, -0.12, 0.0, 0.0),
        Vec2::new(14.0, 0.0),
        0.0,
        slide_weighted(Vec2::new(-8.0, 0.0), FootPart::Flat, 0.3),
        slide_weighted(Vec2::new(36.0, 0.0), FootPart::Flat, 0.7),
        Easing::EaseOut,
    ),
    frame(
        0.55,
        body(-0.3, 12.0, 0.0, 0.18, -0.1, 0.0, 0.0),
        Vec2::new(34.0, 0.0),
        0.0,
        slide_weighted(Vec2::new(12.0, 0.0), FootPart::Flat, 0.7),
        slide_weighted(Vec2::new(58.0, 0.0), FootPart::Flat, 0.3),
        Easing::EaseInOut,
    ),
    frame(
        0.82,
        body(-0.16, 8.0, 0.0, 0.08, -0.02, 0.0, 0.0),
        Vec2::new(48.0, 0.0),
        0.0,
        slide_weighted(Vec2::new(26.0, 0.0), FootPart::Flat, 0.5),
        slide_weighted(Vec2::new(70.0, 0.0), FootPart::Flat, 0.5),
        Easing::EaseOut,
    ),
    frame(
        1.0,
        PoseDelta::ZERO,
        Vec2::new(48.0, 0.0),
        0.0,
        plant_weighted(Vec2::new(26.0, 0.0), FootPart::Flat, 0.5),
        plant_weighted(Vec2::new(70.0, 0.0), FootPart::Flat, 0.5),
        Easing::Linear,
    ),
];

const PIVOT: [ActionKeyframe; 5] = [
    frame(
        0.0,
        body(0.0, 5.0, 0.0, 0.0, 0.0, 0.0, 0.0),
        Vec2::ZERO,
        0.0,
        plant(LEFT_HOME),
        plant(RIGHT_HOME),
        Easing::EaseIn,
    ),
    frame(
        0.25,
        body(-0.2, 10.0, 0.0, 0.2, 0.0, 0.0, 0.0),
        Vec2::ZERO,
        PI * 0.5,
        swing(Vec2::new(-14.0, 0.0), 15.0),
        plant(RIGHT_HOME),
        Easing::EaseOut,
    ),
    frame(
        0.5,
        body(0.15, 8.0, 0.0, 0.0, 0.2, 0.0, 0.0),
        Vec2::ZERO,
        PI,
        plant(LEFT_HOME),
        swing(Vec2::new(14.0, 0.0), 15.0),
        Easing::EaseInOut,
    ),
    frame(
        0.75,
        body(-0.12, 7.0, 0.0, 0.1, 0.0, 0.0, 0.0),
        Vec2::ZERO,
        PI * 1.5,
        swing(Vec2::new(-14.0, 0.0), 12.0),
        plant(RIGHT_HOME),
        Easing::EaseOut,
    ),
    frame(
        1.0,
        PoseDelta::ZERO,
        Vec2::ZERO,
        TAU,
        plant(LEFT_HOME),
        plant(RIGHT_HOME),
        Easing::Linear,
    ),
];

const MOONWALK: [ActionKeyframe; 7] = [
    frame(
        0.0,
        body(-0.12, 18.0, 0.0, 0.0, 0.12, 0.0, 0.0),
        Vec2::ZERO,
        0.0,
        plant_weighted(LEFT_HOME, FootPart::Flat, 1.0),
        plant_weighted(RIGHT_HOME, FootPart::Flat, 0.0),
        Easing::EaseInOut,
    ),
    frame(
        0.16,
        body(-0.42, 28.0, 0.0, 0.08, 0.24, 0.0, 0.0),
        Vec2::new(-8.0, 0.0),
        0.0,
        plant_weighted(LEFT_HOME, FootPart::Flat, 1.0),
        slide_weighted(Vec2::new(10.0, 0.0), FootPart::Flat, 0.0),
        Easing::EaseOut,
    ),
    frame(
        0.33,
        body(-0.52, 26.0, 0.0, 0.18, 0.0, 0.0, 0.0),
        Vec2::new(-18.0, 0.0),
        0.0,
        slide_weighted(Vec2::new(-42.0, 0.0), FootPart::Flat, 0.0),
        plant_weighted(Vec2::new(10.0, 0.0), FootPart::Flat, 1.0),
        Easing::EaseInOut,
    ),
    frame(
        0.5,
        body(-0.42, 28.0, 0.0, 0.08, 0.24, 0.0, 0.0),
        Vec2::new(-28.0, 0.0),
        0.0,
        plant_weighted(Vec2::new(-42.0, 0.0), FootPart::Flat, 1.0),
        slide_weighted(Vec2::new(-10.0, 0.0), FootPart::Flat, 0.0),
        Easing::EaseInOut,
    ),
    frame(
        0.67,
        body(-0.52, 26.0, 0.0, 0.18, 0.0, 0.0, 0.0),
        Vec2::new(-38.0, 0.0),
        0.0,
        slide_weighted(Vec2::new(-58.0, 0.0), FootPart::Flat, 0.0),
        plant_weighted(Vec2::new(-10.0, 0.0), FootPart::Flat, 1.0),
        Easing::EaseInOut,
    ),
    frame(
        0.84,
        body(-0.4, 20.0, 0.0, 0.08, 0.2, 0.0, 0.0),
        Vec2::new(-48.0, 0.0),
        0.0,
        plant_weighted(Vec2::new(-58.0, 0.0), FootPart::Flat, 1.0),
        slide_weighted(Vec2::new(-34.0, 0.0), FootPart::Flat, 0.0),
        Easing::EaseOut,
    ),
    frame(
        1.0,
        PoseDelta::ZERO,
        Vec2::new(-48.0, 0.0),
        0.0,
        plant_weighted(Vec2::new(-70.0, 0.0), FootPart::Flat, 0.5),
        plant_weighted(Vec2::new(-26.0, 0.0), FootPart::Flat, 0.5),
        Easing::Linear,
    ),
];

const GROUNDED_TAGS: &[ActionTag] = &[ActionTag::Grounded];
const TRAVEL_TAGS: &[ActionTag] = &[ActionTag::Grounded, ActionTag::Traveling];
const ACCENT_TAGS: &[ActionTag] = &[ActionTag::Accent];
const ISOLATION_TAGS: &[ActionTag] = &[ActionTag::Accent, ActionTag::Isolating];
const POSE_TAGS: &[ActionTag] = &[ActionTag::Held];
const BALANCE_TAGS: &[ActionTag] = &[ActionTag::Balance, ActionTag::Held];
const AIRBORNE_TAGS: &[ActionTag] = &[ActionTag::Airborne, ActionTag::Accent];
const SIGNATURE_TAGS: &[ActionTag] = &[
    ActionTag::Grounded,
    ActionTag::Traveling,
    ActionTag::Signature,
];
const FREEZE_POSE: PoseDelta = PoseDelta {
    torso_lean: -0.62,
    squat: 24.0,
    left_shoulder: 1.48,
    right_shoulder: -0.22,
    left_elbow: 0.5,
    right_elbow: 0.1,
    head_snap: 0.24,
    ..PoseDelta::ZERO
};
const CHEST_POP: PoseDelta = PoseDelta {
    chest_pop: 22.0,
    torso_lean: -0.08,
    ..PoseDelta::ZERO
};
const SHOULDER_HIT: PoseDelta = PoseDelta {
    left_shoulder: 0.14,
    right_shoulder: -0.08,
    left_shoulder_lift: 18.0,
    ..PoseDelta::ZERO
};
const HEAD_SNAP: PoseDelta = PoseDelta {
    head_snap: 0.62,
    torso_lean: -0.08,
    ..PoseDelta::ZERO
};
const CROUCH_POSE: PoseDelta = PoseDelta {
    squat: 56.0,
    torso_lean: 0.2,
    left_shoulder: 0.35,
    right_shoulder: 0.35,
    ..PoseDelta::ZERO
};
const LEAN_POSE: PoseDelta = PoseDelta {
    // MJ-style whole-body ankle pivot, not a torso bend.
    ankle_lean: -0.34,
    left_shoulder: 0.16,
    right_shoulder: -0.08,
    ..PoseDelta::ZERO
};
const TOE_STAND_POSE: PoseDelta = PoseDelta {
    pelvis_lift: 13.0,
    left_toe_pitch: -0.72,
    right_toe_pitch: -0.72,
    torso_lean: -0.08,
    ..PoseDelta::ZERO
};

static ACTION_CATALOG: [ActionDefinition; 19] = [
    footwork(ActionId::StepTouch, &STEP_TOUCH, GROUNDED_TAGS, 0.75),
    footwork(ActionId::Shuffle, &SHUFFLE, TRAVEL_TAGS, 0.85),
    footwork(ActionId::Stomp, &STOMP, ACCENT_TAGS, 0.6),
    footwork(ActionId::HeelToe, &HEEL_TOE, GROUNDED_TAGS, 0.75),
    footwork(ActionId::CrossStep, &CROSS_STEP, GROUNDED_TAGS, 0.78),
    footwork(ActionId::Backstep, &BACKSTEP, TRAVEL_TAGS, 0.68),
    footwork(ActionId::KneeLift, &KNEE_LIFT, GROUNDED_TAGS, 0.78),
    footwork(ActionId::KickStep, &KICK_STEP, ACCENT_TAGS, 0.72),
    footwork(ActionId::Jump, &JUMP, AIRBORNE_TAGS, 0.68),
    footwork(ActionId::Slide, &SLIDE, TRAVEL_TAGS, 0.75),
    footwork(ActionId::Pivot, &PIVOT, TRAVEL_TAGS, 0.9),
    footwork(ActionId::Moonwalk, &MOONWALK, SIGNATURE_TAGS, 0.875),
    pulse(ActionId::ChestPop, CHEST_POP, ISOLATION_TAGS),
    pulse(ActionId::ShoulderHit, SHOULDER_HIT, ISOLATION_TAGS),
    pulse(ActionId::HeadSnap, HEAD_SNAP, ISOLATION_TAGS),
    pose(ActionId::Freeze, FREEZE_POSE, POSE_TAGS, true),
    pose(ActionId::Crouch, CROUCH_POSE, POSE_TAGS, false),
    pose(ActionId::Lean, LEAN_POSE, POSE_TAGS, false),
    pose(ActionId::ToeStand, TOE_STAND_POSE, BALANCE_TAGS, false),
];

const PHRASE_EVENTS: [ActionInstance; 26] = [
    event(ActionId::StepTouch, 0.0, 2.0, Side::Right),
    event(ActionId::StepTouch, 2.0, 2.0, Side::Left),
    event(ActionId::HeelToe, 4.0, 2.0, Side::Right),
    event(ActionId::HeelToe, 6.0, 2.0, Side::Left),
    event(ActionId::CrossStep, 8.0, 4.0, Side::Right),
    event(ActionId::Backstep, 12.0, 4.0, Side::Right),
    event(ActionId::Shuffle, 16.0, 4.0, Side::Both),
    event(ActionId::KneeLift, 20.0, 2.0, Side::Right),
    event(ActionId::KickStep, 22.0, 2.0, Side::Right),
    event(ActionId::Stomp, 24.0, 2.0, Side::Right),
    event(ActionId::Slide, 30.0, 4.0, Side::Both),
    event(ActionId::Pivot, 34.0, 8.0, Side::Both),
    event(ActionId::Moonwalk, 42.0, 8.0, Side::Both),
    event(ActionId::Jump, 50.0, 2.0, Side::Both),
    event(ActionId::ChestPop, 1.0, 1.0, Side::Both),
    event(ActionId::ShoulderHit, 2.0, 1.0, Side::Right),
    event(ActionId::ChestPop, 5.0, 1.0, Side::Both),
    event(ActionId::ShoulderHit, 7.0, 1.0, Side::Left),
    event(ActionId::HeadSnap, 14.0, 0.8, Side::Right),
    event(ActionId::Crouch, 22.0, 4.0, Side::Both),
    event(ActionId::Freeze, 26.0, 4.0, Side::Both),
    event(ActionId::Lean, 52.0, 4.0, Side::Both),
    event(ActionId::ShoulderHit, 35.0, 1.0, Side::Right),
    event(ActionId::HeadSnap, 46.0, 0.8, Side::Left),
    event(ActionId::ChestPop, 52.0, 1.0, Side::Both),
    event(ActionId::ToeStand, 56.0, 2.0, Side::Both),
];

pub(crate) fn pilot_phrase() -> Phrase {
    Phrase {
        generated: false,
        beats: PILOT_PHRASE_BEATS,
        events: PHRASE_EVENTS.to_vec(),
    }
}

pub(crate) fn walkthrough_moves() -> Vec<WalkthroughMove> {
    ACTION_CATALOG
        .iter()
        .map(|action| {
            let source_event = PHRASE_EVENTS
                .iter()
                .find(|event| event.id == action.id)
                .expect("each catalog action needs an authored phrase duration");
            WalkthroughMove {
                id: action.id,
                name: action_name(action.id),
                beats: source_event.beats,
                side: source_event.side,
            }
        })
        .collect()
}

/// Deterministic hash in [0, 1), standing in for an RNG so phrases stay reproducible.
fn hash01(seed: u64) -> f32 {
    let mut hash = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    hash = (hash ^ (hash >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    hash = (hash ^ (hash >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    hash ^= hash >> 31;
    ((hash >> 11) as f64 / (1u64 << 53) as f64) as f32
}

fn stream_seed(a: u64, b: u64, salt: u64) -> u64 {
    a.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(b.wrapping_mul(0xBF58_476D_1CE4_E5B9))
        .wrapping_add(salt)
}

fn authored_event(id: ActionId) -> &'static ActionInstance {
    PHRASE_EVENTS
        .iter()
        .find(|event| event.id == id)
        .expect("every catalog action must have an authored phrase event")
}

fn footwork_candidates() -> Vec<&'static ActionDefinition> {
    ACTION_CATALOG
        .iter()
        .filter(|action| action.layer == ActionLayer::Footwork)
        .collect()
}

fn pose_candidates() -> Vec<&'static ActionDefinition> {
    ACTION_CATALOG
        .iter()
        .filter(|action| action.layer == ActionLayer::Pose)
        .collect()
}

fn accent_candidates() -> Vec<&'static ActionDefinition> {
    ACTION_CATALOG
        .iter()
        .filter(|action| action.layer == ActionLayer::Accent)
        .collect()
}

/// Samples a footwork move at the given phase and returns the world-space foot
/// anchors, assuming it starts from `entry_feet`.
fn is_both_sided(id: ActionId) -> bool {
    matches!(
        id,
        ActionId::Shuffle
            | ActionId::Slide
            | ActionId::Pivot
            | ActionId::Moonwalk
            | ActionId::Jump
            | ActionId::Stomp
    )
}

/// Samples a footwork move at the given phase and returns the world-space foot
/// anchors, assuming it starts from `entry_feet`.
fn footwork_exit_state(
    action: &ActionDefinition,
    side: Side,
    phase: f32,
    entry_feet: [Vec2; 2],
) -> [Vec2; 2] {
    let sample = sample_footwork(action, phase, side);
    [0, 1].map(|index| entry_feet[index].add(sample.feet[index].position.sub(HOME_FEET[index])))
}

fn squared_distance(a: Vec2, b: Vec2) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}

/// Generate a deterministic, continuous dance phrase from `seed`.
/// Footwork retains its exit state; the final four beats close the loop.
/// Target lengths are rounded up to four beats to keep the groove seamless.
pub(crate) fn generate_phrase(seed: u64, target_beats: f32) -> Phrase {
    let target_beats = if target_beats.is_finite() {
        (target_beats.max(8.0) / 4.0).ceil() * 4.0
    } else {
        256.0
    };
    let mut events: Vec<ActionInstance> = Vec::new();
    let mut beat = 0.0_f32;
    let mut entry_feet = HOME_FEET;
    let mut entry_turn = 0.0_f32;
    let mut previous_id: Option<ActionId> = None;
    let mut index = 0_u64;

    let footwork = footwork_candidates();
    let reserve_beats = 4.0_f32;

    while beat < target_beats - reserve_beats {
        let choice_seed = stream_seed(seed, index, 1);
        let mut choice = (hash01(choice_seed) * footwork.len() as f32) as usize;
        // Avoid immediate repeats.
        if footwork.len() > 1 {
            while Some(footwork[choice].id) == previous_id {
                choice = (choice + 1) % footwork.len();
            }
        }
        let action = &footwork[choice];
        previous_id = Some(action.id);

        let side = if is_both_sided(action.id) {
            Side::Both
        } else if hash01(stream_seed(seed, index, 2)) < 0.5 {
            Side::Right
        } else {
            Side::Left
        };

        let authored = authored_event(action.id);
        let duration = authored.beats.min(target_beats - reserve_beats - beat);
        let event = ActionInstance {
            id: action.id,
            start_beat: beat,
            beats: duration,
            side,
            intensity: 1.0,
            travel: 1.0,
            entry_feet,
            entry_turn,
        };
        events.push(event);

        entry_feet = footwork_exit_state(action, side, action.exit_phase, entry_feet);
        let exit_sample = sample_footwork(action, action.exit_phase, side);
        entry_turn += exit_sample.turn_angle;

        beat += duration;
        index += 1;
    }

    // Closing move: pick the footwork action whose natural exit displacement
    // brings the feet closest back to HOME_FEET.
    let mut best: Option<(ActionId, Side, [Vec2; 2], f32)> = None;
    let mut best_score = f32::INFINITY;
    for action in &footwork {
        for side in [Side::Right, Side::Left, Side::Both] {
            if is_both_sided(action.id) && side != Side::Both {
                continue;
            }
            let exit = footwork_exit_state(action, side, action.exit_phase, entry_feet);
            let score =
                squared_distance(exit[0], HOME_FEET[0]) + squared_distance(exit[1], HOME_FEET[1]);
            if score < best_score {
                best_score = score;
                best = Some((action.id, side, entry_feet, entry_turn));
            }
        }
    }

    if let Some((id, side, close_entry_feet, close_entry_turn)) = best {
        events.push(ActionInstance {
            id,
            start_beat: beat,
            beats: reserve_beats,
            side,
            intensity: 1.0,
            travel: 1.0,
            entry_feet: close_entry_feet,
            entry_turn: close_entry_turn,
        });
        beat += reserve_beats;
    }

    // Sprinkle accents and poses on top.
    let accents = accent_candidates();
    let poses = pose_candidates();
    let mut accent_index = 0_u64;
    let mut pose_index = 0_u64;
    let accent_grid = 1.0_f32;
    let mut beat_marker = 1.0_f32;
    while beat_marker < beat - reserve_beats {
        if hash01(stream_seed(seed, accent_index, 7)) < 0.35 {
            let choice = (hash01(stream_seed(seed, accent_index, 8)) * accents.len() as f32)
                as usize
                % accents.len();
            let action = accents[choice];
            let authored = authored_event(action.id);
            events.push(ActionInstance {
                id: action.id,
                start_beat: beat_marker,
                beats: authored.beats.min(1.0),
                side: if hash01(stream_seed(seed, accent_index, 9)) < 0.5 {
                    Side::Right
                } else {
                    Side::Left
                },
                intensity: 1.0,
                travel: 1.0,
                entry_feet: HOME_FEET,
                entry_turn: 0.0,
            });
        }
        accent_index += 1;
        beat_marker += accent_grid;
    }

    let mut pose_beat = 8.0_f32;
    while pose_beat < beat - 8.0 {
        if hash01(stream_seed(seed, pose_index, 10)) < 0.4 {
            let choice = (hash01(stream_seed(seed, pose_index, 11)) * poses.len() as f32) as usize
                % poses.len();
            let action = poses[choice];
            let authored = authored_event(action.id);
            let duration = (authored.beats * 0.5).clamp(2.0, 6.0);
            events.push(ActionInstance {
                id: action.id,
                start_beat: pose_beat,
                beats: duration,
                side: Side::Both,
                intensity: 1.0,
                travel: 1.0,
                entry_feet: HOME_FEET,
                entry_turn: 0.0,
            });
            pose_beat += duration;
        } else {
            pose_beat += 4.0;
        }
        pose_index += 1;
    }

    Phrase {
        generated: true,
        beats: beat,
        events,
    }
}

pub(crate) fn sample_walkthrough_move(cue: WalkthroughMove, phase: f32) -> MotionSample {
    let action = definition(cue.id);
    let phase = phase.clamp(0.0, 1.0);
    match action.shape {
        ActionShape::Footwork(_) => {
            let sample = sample_footwork(action, phase, cue.side);
            let mut pose = NEUTRAL_POSE;
            sample.body.add_to(&mut pose, 1.0);
            MotionSample {
                state: MotionState {
                    pose,
                    root: sample.root,
                    turn_angle: sample.turn_angle,
                },
                feet: sample.feet,
            }
        }
        ActionShape::Pulse(target) | ActionShape::Hold(target) => {
            let event = ActionInstance {
                id: cue.id,
                start_beat: 0.0,
                beats: cue.beats,
                side: cue.side,
                intensity: 1.0,
                travel: 1.0,
                entry_feet: HOME_FEET,
                entry_turn: 0.0,
            };
            let weight = event_weight(event, phase * cue.beats).unwrap_or(0.0);
            let target = if cue.side == Side::Left {
                target.mirrored()
            } else {
                target
            };
            let mut pose = NEUTRAL_POSE;
            target.add_to(&mut pose, weight);
            MotionSample {
                state: MotionState {
                    pose,
                    root: Vec2::ZERO,
                    turn_angle: 0.0,
                },
                feet: MotionSample::NEUTRAL.feet,
            }
        }
    }
}

fn action_name(id: ActionId) -> &'static str {
    match id {
        ActionId::StepTouch => "Step-touch",
        ActionId::Shuffle => "Shuffle",
        ActionId::Stomp => "Stomp",
        ActionId::HeelToe => "Heel-toe",
        ActionId::CrossStep => "Cross-step",
        ActionId::Backstep => "Backstep",
        ActionId::KneeLift => "Knee lift",
        ActionId::KickStep => "Kick-step",
        ActionId::Jump => "Jump",
        ActionId::Slide => "Slide",
        ActionId::Pivot => "Pivot",
        ActionId::Moonwalk => "Moonwalk",
        ActionId::ChestPop => "Chest pop",
        ActionId::ShoulderHit => "Shoulder hit",
        ActionId::HeadSnap => "Head snap",
        ActionId::Freeze => "Freeze",
        ActionId::Crouch => "Crouch",
        ActionId::Lean => "Lean",
        ActionId::ToeStand => "Toe stand",
    }
}

fn definition(id: ActionId) -> &'static ActionDefinition {
    let definition = ACTION_CATALOG
        .iter()
        .find(|definition| definition.id == id)
        .expect("every choreography event must have an action definition");
    debug_assert!(!definition.tags.is_empty());
    definition
}

/// Samples a layered phrase at an unwrapped beat time. Footwork events own the
/// feet/root, accents and poses add body channels, and groove remains underneath.
pub(crate) fn sample_phrase(phrase: &Phrase, beat: f64) -> MotionSample {
    if !phrase.beats.is_finite() || phrase.beats <= 0.0 {
        return MotionSample::NEUTRAL;
    }
    let beat = if beat.is_finite() { beat } else { 0.0 };
    let cycle = (beat / f64::from(phrase.beats)).floor();
    let local_beat = beat.rem_euclid(f64::from(phrase.beats)) as f32;
    let cycle_root = phrase_root_delta(phrase).scale(cycle as f32);
    let cycle_turn = phrase_turn_delta(phrase) * cycle as f32;

    let freeze = phrase
        .events
        .iter()
        .filter(|event| definition(event.id).freezes_groove)
        .filter_map(|event| {
            event_weight(*event, local_beat)
                .map(|weight| (weight * event.intensity).clamp(0.0, 1.0))
        })
        .fold(0.0, f32::max);
    let mut pose = NEUTRAL_POSE;
    groove_delta(local_beat).add_to(&mut pose, 1.0 - freeze);

    // Authored phrases accumulate completed travel; generated phrases carry
    // explicit entry states and close their travel at the end of each loop.
    let authored_mode = !phrase.generated;

    let mut root_base = cycle_root;
    let mut turn_base = cycle_turn;
    let mut active_footwork = None;
    for event in &phrase.events {
        let action = definition(event.id);
        if action.layer != ActionLayer::Footwork {
            continue;
        }
        let end = event.start_beat + event.beats;
        if authored_mode && local_beat >= end {
            let terminal = sample_footwork(action, 1.0, event.side);
            root_base = root_base.add(terminal.root.scale(event.travel));
            turn_base += terminal.turn_angle * event.intensity;
        } else if local_beat >= event.start_beat && event.beats > 0.0 {
            active_footwork = Some((*event, action));
        }
    }

    let mut feet = if authored_mode {
        [
            FootSample {
                position: LEFT_HOME.add(root_base),
                lift: 0.0,
                contact: FootContact::Planted,
                part: FootPart::Flat,
                weight: 0.5,
            },
            FootSample {
                position: RIGHT_HOME.add(root_base),
                lift: 0.0,
                contact: FootContact::Planted,
                part: FootPart::Flat,
                weight: 0.5,
            },
        ]
    } else {
        MotionSample::NEUTRAL.feet
    };
    let mut root = root_base;
    let mut turn_angle = turn_base;
    if let Some((event, action)) = active_footwork {
        let phase = ((local_beat - event.start_beat) / event.beats).clamp(0.0, 1.0);
        let sample = sample_footwork(action, phase, event.side);
        if authored_mode {
            sample.body.add_to(&mut pose, event.intensity);
            root = root_base.add(sample.root.scale(event.travel));
            turn_angle = turn_base + sample.turn_angle * event.intensity;
            feet = [0, 1].map(|index| {
                let foot_sample = sample.feet[index];
                let home = HOME_FEET[index];
                FootSample {
                    position: root_base
                        .add(home)
                        .add(foot_sample.position.sub(home).scale(event.travel)),
                    lift: foot_sample.lift * event.intensity,
                    contact: foot_sample.contact,
                    part: foot_sample.part,
                    weight: foot_sample.weight,
                }
            });
        } else {
            let chained = sample_generated_footwork(phrase, local_beat);
            pose = chained.state.pose;
            groove_delta(local_beat).add_to(&mut pose, 1.0 - freeze);
            root = chained.state.root;
            turn_angle = chained.state.turn_angle + cycle_turn;
            feet = chained.feet;
        }
    }

    for event in &phrase.events {
        let action = definition(event.id);
        if action.layer == ActionLayer::Footwork {
            continue;
        }
        let Some(weight) = event_weight(*event, local_beat) else {
            continue;
        };
        let (ActionShape::Pulse(target) | ActionShape::Hold(target)) = action.shape else {
            continue;
        };
        let target = if event.side == Side::Left {
            target.mirrored()
        } else {
            target
        };
        target.add_to(&mut pose, weight * event.intensity);
    }

    let (groove_x, groove_y) = groove_root(local_beat);
    MotionSample {
        state: MotionState {
            pose,
            root: root.add(Vec2::new(groove_x, groove_y).scale(1.0 - freeze)),
            turn_angle,
        },
        feet,
    }
}

/// Convert a move's local output into its chained stage coordinates.
fn chained_move(event: ActionInstance, phase: f32) -> MotionSample {
    let action = definition(event.id);
    let sample = sample_footwork_with_blending(action, phase * action.exit_phase, event.side, true);
    let mut pose = NEUTRAL_POSE;
    sample.body.add_to(&mut pose, event.intensity);
    MotionSample {
        state: MotionState {
            pose,
            root: midpoint(event.entry_feet[0], event.entry_feet[1])
                .add(sample.root.scale(event.travel)),
            turn_angle: event.entry_turn + sample.turn_angle * event.intensity,
        },
        feet: [0, 1].map(|index| FootSample {
            position: event.entry_feet[index].add(
                sample.feet[index]
                    .position
                    .sub(HOME_FEET[index])
                    .scale(event.travel),
            ),
            lift: sample.feet[index].lift * event.intensity,
            ..sample.feet[index]
        }),
    }
}

fn sample_generated_footwork(phrase: &Phrase, beat: f32) -> MotionSample {
    let mut footwork = phrase
        .events
        .iter()
        .copied()
        .filter(|event| definition(event.id).layer == ActionLayer::Footwork);
    let Some(first) = footwork.next() else {
        return MotionSample::NEUTRAL;
    };
    let mut previous = None;
    let mut event = first;
    while beat >= event.start_beat + event.beats {
        previous = Some(event);
        let Some(next) = footwork.next() else {
            return MotionSample::NEUTRAL;
        };
        event = next;
    }
    if beat < event.start_beat {
        return MotionSample::NEUTRAL;
    }
    let phase = ((beat - event.start_beat) / event.beats).clamp(0.0, 1.0);
    let mut sample = chained_move(event, phase);
    if let Some(previous) = previous {
        // Carry pose, foot lift and root as well as foot anchors across the boundary.
        let previous = chained_move(previous, 1.0);
        let blend = smoothstep(((beat - event.start_beat) / 0.5).clamp(0.0, 1.0));
        sample = blend_samples(previous, sample, blend);
    }
    if event.start_beat + event.beats == phrase.beats {
        let mut start = chained_move(first, 0.0);
        start.state.turn_angle = generated_turn_delta(phrase);
        // A deliberate closing slide returns to the starting stance, with no teleport.
        sample = blend_samples(sample, start, smoothstep(phase));
    }
    sample
}

fn generated_turn_delta(phrase: &Phrase) -> f32 {
    let Some(last) = phrase
        .events
        .iter()
        .rev()
        .find(|event| definition(event.id).layer == ActionLayer::Footwork)
    else {
        return 0.0;
    };
    (chained_move(*last, 1.0).state.turn_angle / TAU).round() * TAU
}

fn midpoint(a: Vec2, b: Vec2) -> Vec2 {
    Vec2::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5)
}

fn event_weight(event: ActionInstance, beat: f32) -> Option<f32> {
    if event.beats <= 0.0 || beat < event.start_beat || beat >= event.start_beat + event.beats {
        return None;
    }
    let phase = ((beat - event.start_beat) / event.beats).clamp(0.0, 1.0);
    let action = definition(event.id);
    let weight = match action.shape {
        // Onset-peaked envelope: maximum at the start of the event, decaying
        // toward the end so accents land on their authored beat.
        ActionShape::Pulse(_) => (PI * 0.5 * (1.0 - phase)).sin().powi(2),
        ActionShape::Hold(_) => {
            let edge = 0.16_f32.min(0.5 / event.beats.max(0.5));
            smoothstep((phase / edge).clamp(0.0, 1.0))
                * smoothstep(((1.0 - phase) / edge).clamp(0.0, 1.0))
        }
        ActionShape::Footwork(_) => 1.0,
    };
    Some(weight)
}

#[derive(Clone, Copy)]
struct FootworkSample {
    body: PoseDelta,
    root: Vec2,
    turn_angle: f32,
    feet: [FootSample; 2],
}

fn sample_footwork(action: &ActionDefinition, phase: f32, side: Side) -> FootworkSample {
    sample_footwork_with_blending(action, phase, side, false)
}

fn sample_footwork_with_blending(
    action: &ActionDefinition,
    phase: f32,
    side: Side,
    continuous: bool,
) -> FootworkSample {
    let ActionShape::Footwork(keyframes) = action.shape else {
        unreachable!("only footwork actions have keyframes")
    };
    let mut sample = sample_keyframes(keyframes, phase, continuous);
    if side == Side::Left {
        sample.body = sample.body.mirrored();
        sample.root.x = -sample.root.x;
        sample.turn_angle = -sample.turn_angle;
        sample.feet = [mirror_foot(sample.feet[1]), mirror_foot(sample.feet[0])];
    }
    sample
}

fn mirror_foot(mut sample: FootSample) -> FootSample {
    sample.position.x = -sample.position.x;
    sample
}

fn sample_keyframes(keyframes: &[ActionKeyframe], phase: f32, continuous: bool) -> FootworkSample {
    let phase = phase.clamp(0.0, 1.0);
    let Some(pair) = keyframes.windows(2).find(|pair| phase <= pair[1].phase) else {
        let last = *keyframes
            .last()
            .expect("footwork definitions need keyframes");
        return FootworkSample {
            body: last.body,
            root: last.root,
            turn_angle: last.turn_angle,
            feet: last.feet.map(resolve_foot),
        };
    };
    let from = pair[0];
    let to = pair[1];
    let interval = to.phase - from.phase;
    let easing = if continuous && from.easing == Easing::Hold {
        Easing::EaseInOut
    } else {
        from.easing
    };
    let t = easing.apply((phase - from.phase) / interval);
    FootworkSample {
        body: interpolate_delta(from.body, to.body, t),
        root: from.root.lerp(to.root, t),
        turn_angle: lerp(from.turn_angle, to.turn_angle, t),
        feet: [
            blend_foot(resolve_foot(from.feet[0]), resolve_foot(to.feet[0]), t),
            blend_foot(resolve_foot(from.feet[1]), resolve_foot(to.feet[1]), t),
        ],
    }
}

fn resolve_foot(intent: FootIntent) -> FootSample {
    match intent {
        FootIntent::Plant {
            anchor,
            part,
            weight,
        } => FootSample {
            position: anchor,
            lift: 0.0,
            contact: FootContact::Planted,
            part,
            weight,
        },
        FootIntent::Swing { target, lift } => FootSample {
            position: target,
            lift,
            contact: FootContact::Swinging,
            part: FootPart::Flat,
            weight: 0.0,
        },
        FootIntent::Slide {
            target,
            part,
            weight,
        } => FootSample {
            position: target,
            lift: 0.0,
            contact: FootContact::Sliding,
            part,
            weight,
        },
    }
}

fn blend_foot(from: FootSample, to: FootSample, t: f32) -> FootSample {
    let same_plant = from.contact == FootContact::Planted
        && to.contact == FootContact::Planted
        && near(from.position, to.position)
        && from.part == to.part
        && (from.weight - to.weight).abs() < 0.001;
    if same_plant {
        return from;
    }
    FootSample {
        position: from.position.lerp(to.position, t),
        lift: lerp(from.lift, to.lift, t),
        contact: if t < 0.5 { from.contact } else { to.contact },
        part: if t < 0.5 { from.part } else { to.part },
        weight: lerp(from.weight, to.weight, t),
    }
}

fn groove_delta(beat: f32) -> PoseDelta {
    let sway = (beat * PI * 0.5).sin();
    let bounce = (beat * PI * 0.5).cos();
    PoseDelta {
        torso_lean: 0.055 * sway,
        squat: 3.5 + 2.0 * (1.0 - bounce),
        left_shoulder: 0.045 * sway,
        right_shoulder: -0.045 * sway,
        head_snap: 0.035 * sway,
        ..PoseDelta::ZERO
    }
}

fn groove_root(beat: f32) -> (f32, f32) {
    (
        (beat * PI * 0.5).sin() * 3.0,
        (1.0 - (beat * PI * 0.5).cos()) * 1.5,
    )
}

fn phrase_root_delta(phrase: &Phrase) -> Vec2 {
    if phrase.generated {
        return Vec2::ZERO;
    }
    phrase
        .events
        .iter()
        .filter(|event| definition(event.id).layer == ActionLayer::Footwork)
        .fold(Vec2::ZERO, |root, event| {
            let end = sample_footwork(definition(event.id), 1.0, event.side).root;
            root.add(end.scale(event.travel))
        })
}

fn phrase_turn_delta(phrase: &Phrase) -> f32 {
    if phrase.generated {
        return generated_turn_delta(phrase);
    }
    phrase
        .events
        .iter()
        .filter(|event| definition(event.id).layer == ActionLayer::Footwork)
        .map(|event| {
            sample_footwork(definition(event.id), 1.0, event.side).turn_angle * event.intensity
        })
        .sum()
}

fn interpolate_delta(from: PoseDelta, to: PoseDelta, t: f32) -> PoseDelta {
    PoseDelta {
        torso_lean: lerp(from.torso_lean, to.torso_lean, t),
        squat: lerp(from.squat, to.squat, t),
        pelvis_lift: lerp(from.pelvis_lift, to.pelvis_lift, t),
        ankle_lean: lerp(from.ankle_lean, to.ankle_lean, t),
        left_shoulder: lerp(from.left_shoulder, to.left_shoulder, t),
        right_shoulder: lerp(from.right_shoulder, to.right_shoulder, t),
        left_elbow: lerp(from.left_elbow, to.left_elbow, t),
        right_elbow: lerp(from.right_elbow, to.right_elbow, t),
        chest_pop: lerp(from.chest_pop, to.chest_pop, t),
        left_shoulder_lift: lerp(from.left_shoulder_lift, to.left_shoulder_lift, t),
        right_shoulder_lift: lerp(from.right_shoulder_lift, to.right_shoulder_lift, t),
        head_snap: lerp(from.head_snap, to.head_snap, t),
        left_toe_pitch: lerp(from.left_toe_pitch, to.left_toe_pitch, t),
        right_toe_pitch: lerp(from.right_toe_pitch, to.right_toe_pitch, t),
    }
}

pub(crate) fn blend_samples(from: MotionSample, to: MotionSample, t: f32) -> MotionSample {
    let t = t.clamp(0.0, 1.0);
    MotionSample {
        state: MotionState {
            pose: interpolate_pose(from.state.pose, to.state.pose, t),
            root: from.state.root.lerp(to.state.root, t),
            turn_angle: lerp(from.state.turn_angle, to.state.turn_angle, t),
        },
        feet: [
            blend_foot(from.feet[0], to.feet[0], t),
            blend_foot(from.feet[1], to.feet[1], t),
        ],
    }
}

fn interpolate_pose(from: DancePose, to: DancePose, t: f32) -> DancePose {
    DancePose {
        torso_lean: lerp(from.torso_lean, to.torso_lean, t),
        squat: lerp(from.squat, to.squat, t),
        pelvis_lift: lerp(from.pelvis_lift, to.pelvis_lift, t),
        ankle_lean: lerp(from.ankle_lean, to.ankle_lean, t),
        left_shoulder: lerp(from.left_shoulder, to.left_shoulder, t),
        right_shoulder: lerp(from.right_shoulder, to.right_shoulder, t),
        left_elbow: lerp(from.left_elbow, to.left_elbow, t),
        right_elbow: lerp(from.right_elbow, to.right_elbow, t),
        chest_pop: lerp(from.chest_pop, to.chest_pop, t),
        left_shoulder_lift: lerp(from.left_shoulder_lift, to.left_shoulder_lift, t),
        right_shoulder_lift: lerp(from.right_shoulder_lift, to.right_shoulder_lift, t),
        head_snap: lerp(from.head_snap, to.head_snap, t),
        left_toe_pitch: lerp(from.left_toe_pitch, to.left_toe_pitch, t),
        right_toe_pitch: lerp(from.right_toe_pitch, to.right_toe_pitch, t),
    }
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn near(a: Vec2, b: Vec2) -> bool {
    (a.x - b.x).abs() < 0.001 && (a.y - b.y).abs() < 0.001
}

fn lerp(from: f32, to: f32, t: f32) -> f32 {
    from + (to - from) * t
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_at(beat: f32) -> MotionSample {
        sample_phrase(&pilot_phrase(), f64::from(beat))
    }

    #[test]
    fn catalog_separates_footwork_accents_and_poses() {
        assert_eq!(ACTION_CATALOG.len(), 19);
        for id in [
            ActionId::StepTouch,
            ActionId::Shuffle,
            ActionId::HeelToe,
            ActionId::CrossStep,
            ActionId::Backstep,
            ActionId::KneeLift,
            ActionId::KickStep,
            ActionId::Jump,
            ActionId::Stomp,
            ActionId::Slide,
            ActionId::Pivot,
            ActionId::Moonwalk,
        ] {
            assert_eq!(definition(id).layer, ActionLayer::Footwork);
        }
        for id in [
            ActionId::ChestPop,
            ActionId::ShoulderHit,
            ActionId::HeadSnap,
        ] {
            assert_eq!(definition(id).layer, ActionLayer::Accent);
        }
        for id in [
            ActionId::Freeze,
            ActionId::Crouch,
            ActionId::Lean,
            ActionId::ToeStand,
        ] {
            assert_eq!(definition(id).layer, ActionLayer::Pose);
        }
        assert!(ACTION_CATALOG.iter().all(|action| !action.tags.is_empty()));
        assert!(
            ACTION_CATALOG
                .iter()
                .all(|action| { PHRASE_EVENTS.iter().any(|event| event.id == action.id) })
        );
    }

    #[test]
    fn walkthrough_lists_each_catalog_move_once_with_its_authored_duration() {
        let cues = walkthrough_moves();
        assert_eq!(cues.len(), ACTION_CATALOG.len());
        for (index, cue) in cues.iter().enumerate() {
            assert_eq!(cue.id, ACTION_CATALOG[index].id);
            let authored = PHRASE_EVENTS
                .iter()
                .find(|event| event.id == cue.id)
                .expect("catalog action should have an authored event");
            assert_eq!(cue.beats, authored.beats);
            assert!(!cue.name.is_empty());
            assert!(cue.beats > 0.0);
            assert!(
                cues[..index].iter().all(|previous| previous.id != cue.id),
                "walkthrough should include each action only once"
            );
            for phase in [0.0, 0.5, 1.0] {
                let sample = sample_walkthrough_move(*cue, phase);
                assert!(sample.state.root.x.is_finite());
                assert!(sample.state.root.y.is_finite());
                assert!(sample.state.turn_angle.is_finite());
            }
        }
    }

    #[test]
    fn choreography_uses_short_varied_durations_and_overlapping_layers() {
        let durations: Vec<_> = PHRASE_EVENTS.iter().map(|event| event.beats).collect();
        assert!(durations.iter().any(|beats| *beats <= 0.8));
        assert!(durations.contains(&0.8));
        assert!(durations.contains(&1.0));
        assert!(durations.contains(&2.0));
        assert!(durations.contains(&4.0));
        assert!(durations.contains(&8.0));
        assert!(durations.contains(&8.0));
        assert!(
            PHRASE_EVENTS
                .iter()
                .any(|event| { event.id == ActionId::ChestPop && event.start_beat == 1.0 })
        );
        assert!(
            PHRASE_EVENTS
                .iter()
                .any(|event| { event.id == ActionId::StepTouch && event.start_beat == 0.0 })
        );

        let mut footwork: Vec<_> = PHRASE_EVENTS
            .iter()
            .filter(|event| definition(event.id).layer == ActionLayer::Footwork)
            .map(|event| (event.start_beat, event.start_beat + event.beats))
            .collect();
        footwork.sort_by(|left, right| left.0.total_cmp(&right.0));
        assert!(footwork.windows(2).all(|pair| pair[0].1 <= pair[1].0));
    }

    #[test]
    fn continuous_groove_runs_under_actions_and_keeps_support_feet_grounded() {
        let on_beat = sample_at(0.0);
        let off_beat = sample_at(0.5);
        assert_ne!(on_beat.state.pose, off_beat.state.pose);
        assert!(on_beat.state.pose.squat > 0.0);
        assert!(off_beat.state.pose.squat > 0.0);
        assert!(
            on_beat
                .feet
                .iter()
                .all(|foot| foot.contact == FootContact::Planted)
        );
        // Onset-peaked accent: maximum is near the start of the event.
        let accent = sample_at(2.05);
        assert!(accent.state.pose.left_shoulder > 0.4);
        let step_in_motion = sample_at(3.6);
        assert!(step_in_motion.feet.iter().any(|foot| foot.lift > 0.0));
    }

    #[test]
    fn footwork_primitives_have_distinct_contact_and_pose_shapes() {
        let step = sample_at(1.2);
        // Step-touch has a weight-bearing planted foot and an unloaded touching foot.
        assert!(step.feet.iter().any(|foot| foot.weight > 0.9));
        assert!(step.feet.iter().any(|foot| foot.weight < 0.1));
        let cross = sample_at(9.8);
        assert!(cross.feet[1].position.x < 0.0);
        let shuffle = sample_at(17.6);
        assert!(
            shuffle
                .feet
                .iter()
                .any(|foot| foot.contact == FootContact::Sliding)
        );
        let knee = sample_at(21.0);
        assert!(knee.feet[1].lift > 40.0);
        let kick = sample_at(22.88);
        assert!(kick.feet[1].position.x > 50.0);
        let stomp = sample_at(25.4);
        assert!(stomp.state.pose.squat > 10.0);
        assert!(stomp.feet[1].contact == FootContact::Planted || stomp.feet[1].lift < 10.0);
        let jump = sample_at(51.3);
        assert!(jump.state.pose.pelvis_lift > 50.0);
        assert!(jump.feet.iter().all(|foot| foot.lift > 50.0));
    }

    #[test]
    fn accents_stack_with_footwork_without_becoming_whole_moves() {
        let sample = sample_at(7.05);
        assert!(sample.state.pose.right_shoulder > NEUTRAL_POSE.right_shoulder);
        let light_foot = sample_at(2.5);
        assert!(light_foot.feet.iter().any(|foot| foot.weight < 0.1));
        let shoulder_hit = sample_at(2.1);
        assert!(shoulder_hit.state.pose.left_shoulder > 0.4);
        let chest = sample_at(1.05);
        assert!(chest.state.pose.chest_pop > 8.0);
        let head = sample_at(14.05);
        assert!(head.state.pose.head_snap > 0.5);
    }

    #[test]
    fn freeze_mutes_background_groove_and_toe_stand_has_toe_support() {
        let frozen = sample_at(28.0);
        assert!((frozen.state.pose.squat - FREEZE_POSE.squat).abs() < 1.0e-3);
        assert!(frozen.state.pose.head_snap.abs() > 0.2);
        let toe_stand = sample_at(57.0);
        assert!(toe_stand.state.pose.pelvis_lift > 10.0);
        assert!(toe_stand.state.pose.left_toe_pitch < -0.5);
        assert!(toe_stand.state.pose.right_toe_pitch < -0.5);
    }

    #[test]
    fn heel_toe_and_moonwalk_slides_stay_on_the_floor() {
        let heel_toe = sample_at(5.0);
        // Heel-toe now expresses contact through FootPart, not pose toe pitch.
        assert!(heel_toe.feet[1].part != FootPart::Flat);
        let moonwalk = sample_at(46.0);
        assert!(
            moonwalk
                .feet
                .iter()
                .any(|foot| foot.contact == FootContact::Sliding)
        );
        assert!(moonwalk.feet.iter().all(|foot| foot.lift == 0.0));
        assert!(moonwalk.state.pose.torso_lean < -0.2);
    }

    #[test]
    fn pulse_accent_peaks_at_onset_not_midpoint() {
        let onset = sample_at(1.0 + 0.02);
        let midpoint = sample_at(1.0 + 0.5);
        let end = sample_at(1.0 + 0.98);
        assert!(onset.state.pose.chest_pop > midpoint.state.pose.chest_pop);
        assert!(onset.state.pose.chest_pop > end.state.pose.chest_pop);
    }

    #[test]
    fn step_touch_transfers_weight_to_planted_foot() {
        let step = sample_at(0.45);
        let heavy = step
            .feet
            .iter()
            .max_by(|a, b| a.weight.total_cmp(&b.weight))
            .unwrap();
        let light = step
            .feet
            .iter()
            .min_by(|a, b| a.weight.total_cmp(&b.weight))
            .unwrap();
        assert!(heavy.weight > 0.9);
        assert!(light.weight < 0.1);
        assert!(heavy.contact == FootContact::Planted);
    }

    #[test]
    fn heel_toe_cycles_through_contact_parts() {
        let heel = sample_at(4.25);
        let ball = sample_at(4.7);
        let toe = sample_at(5.25);
        assert_eq!(heel.feet[1].part, FootPart::Heel);
        assert_eq!(ball.feet[1].part, FootPart::Ball);
        assert_eq!(toe.feet[1].part, FootPart::Toe);
    }

    #[test]
    fn moonwalk_unloads_the_sliding_foot() {
        let slide = sample_at(43.3);
        let planted = slide
            .feet
            .iter()
            .find(|f| f.weight > 0.9)
            .expect("planted foot");
        let sliding = slide
            .feet
            .iter()
            .find(|f| f.weight < 0.1)
            .expect("sliding foot");
        assert_eq!(planted.contact, FootContact::Planted);
        assert_eq!(sliding.contact, FootContact::Sliding);
    }

    #[test]
    fn lean_is_a_whole_body_ankle_pivot() {
        let lean = sample_at(53.0);
        assert!(lean.state.pose.ankle_lean.abs() > 0.2);
        assert!(lean.state.pose.torso_lean.abs() < 0.1);
    }

    #[test]
    fn pivot_is_stepped_and_turn_angle_stays_unwrapped() {
        let pivot = sample_at(36.8);
        assert!(pivot.state.turn_angle > 0.0);
        assert!(pivot.feet.iter().any(|foot| foot.lift > 0.0));
        assert!((phrase_turn_delta(&pilot_phrase()) - TAU).abs() < 1.0e-5);
        let second_loop = sample_phrase(&pilot_phrase(), f64::from(PILOT_PHRASE_BEATS) + 36.8);
        assert!(second_loop.state.turn_angle > pivot.state.turn_angle);
    }

    #[test]
    fn footwork_travel_cancels_and_boundary_positions_are_continuous() {
        assert!(near(phrase_root_delta(&pilot_phrase()), Vec2::ZERO));
        let mut boundaries: Vec<f32> = PHRASE_EVENTS
            .iter()
            .filter(|event| definition(event.id).layer == ActionLayer::Footwork)
            .flat_map(|event| [event.start_beat, event.start_beat + event.beats])
            .collect();
        boundaries.sort_by(f32::total_cmp);
        boundaries.dedup_by(|a, b| (*a - *b).abs() < 1.0e-5);
        for boundary in boundaries {
            if boundary <= 0.0 || boundary >= PILOT_PHRASE_BEATS {
                continue;
            }
            let before = sample_at(boundary - 0.001);
            let after = sample_at(boundary);
            assert!((before.state.root.x - after.state.root.x).abs() < 1.0);
            for side in 0..2 {
                assert!((before.feet[side].position.x - after.feet[side].position.x).abs() < 1.0);
            }
        }
    }

    #[test]
    fn phrase_is_sixty_four_beats_and_loops_deterministically() {
        assert_eq!(pilot_phrase().beats, PILOT_PHRASE_BEATS);
        let first = sample_at(21.0);
        let repeat = sample_at(21.0);
        assert_eq!(first, repeat);
        let start = sample_at(0.0);
        let loop_start = sample_at(PILOT_PHRASE_BEATS);
        assert_eq!(start.state.pose, loop_start.state.pose);
        assert_eq!(start.state.root, loop_start.state.root);
        assert!((loop_start.state.turn_angle - start.state.turn_angle - TAU).abs() < 1.0e-4);
        assert_eq!(start.feet, loop_start.feet);
    }

    #[test]
    fn sample_handles_non_finite_time_and_invalid_phrase() {
        assert_eq!(
            sample_phrase(&pilot_phrase(), f64::NAN).state.pose,
            sample_at(0.0).state.pose
        );
        assert_eq!(
            sample_phrase(
                &Phrase {
                    generated: false,
                    beats: 0.0,
                    events: Vec::new(),
                },
                1.0
            ),
            MotionSample::NEUTRAL
        );
    }

    #[test]
    fn generated_phrase_is_deterministic_and_varies_by_seed() {
        let a = generate_phrase(7, 64.0);
        let b = generate_phrase(7, 64.0);
        let c = generate_phrase(42, 64.0);
        assert_eq!(a.events.len(), b.events.len());
        assert!(
            a.events
                .iter()
                .zip(&b.events)
                .all(|(left, right)| left == right)
        );
        assert!(
            a.events
                .iter()
                .zip(&c.events)
                .any(|(left, right)| left != right),
            "different seeds should produce different phrases"
        );
    }

    #[test]
    fn generated_phrase_chains_footwork_without_neutral_resets() {
        let phrase = generate_phrase(3, 128.0);
        let chained = phrase
            .events
            .iter()
            .filter(|event| definition(event.id).layer == ActionLayer::Footwork)
            .any(|event| event.entry_feet != HOME_FEET || event.entry_turn != 0.0);
        assert!(
            chained,
            "procedural phrase should chain at least one footwork move"
        );
    }

    #[test]
    fn generated_phrase_length_is_near_target() {
        let phrase = generate_phrase(11, 128.0);
        assert!(phrase.beats >= 120.0 && phrase.beats <= 140.0);
    }

    #[test]
    fn generated_move_boundaries_preserve_pose_root_turn_and_feet() {
        for seed in [0, 1, 3, 7, 42, u64::MAX] {
            let phrase = generate_phrase(seed, 256.0);
            for event in phrase.events.iter().filter(|event| {
                definition(event.id).layer == ActionLayer::Footwork && event.start_beat > 0.0
            }) {
                let before = sample_generated_footwork(&phrase, event.start_beat - 0.00001);
                let after = sample_generated_footwork(&phrase, event.start_beat);
                assert_sample_near(before, after);
            }
        }
    }

    fn assert_sample_near(a: MotionSample, b: MotionSample) {
        assert!((a.state.root.x - b.state.root.x).abs() < 0.02);
        assert!((a.state.root.y - b.state.root.y).abs() < 0.02);
        assert!((a.state.turn_angle - b.state.turn_angle).abs() < 0.002);
        assert!((a.state.pose.torso_lean - b.state.pose.torso_lean).abs() < 0.002);
        assert!((a.state.pose.squat - b.state.pose.squat).abs() < 0.02);
        assert!((a.state.pose.pelvis_lift - b.state.pose.pelvis_lift).abs() < 0.02);
        assert!((a.state.pose.left_shoulder - b.state.pose.left_shoulder).abs() < 0.002);
        assert!((a.state.pose.right_shoulder - b.state.pose.right_shoulder).abs() < 0.002);
        for (left, right) in a.feet.iter().zip(b.feet) {
            assert!((left.position.x - right.position.x).abs() < 0.02);
            assert!((left.position.y - right.position.y).abs() < 0.02);
            assert!((left.lift - right.lift).abs() < 0.02);
        }
    }

    #[test]
    fn generated_loops_close_without_root_drift_or_foot_teleports() {
        for seed in [0, 1, 3, 7, 42, u64::MAX] {
            let phrase = generate_phrase(seed, 256.0);
            let start = sample_phrase(&phrase, 0.0);
            let mut end = sample_phrase(&phrase, f64::from(phrase.beats) - 0.00001);
            end.state.turn_angle -= generated_turn_delta(&phrase);
            assert_sample_near(start, end);
            for cycle in [1.0, 2.0, 10.0] {
                let mut repeated = sample_phrase(&phrase, f64::from(phrase.beats) * cycle);
                repeated.state.turn_angle -= generated_turn_delta(&phrase) * cycle as f32;
                assert_sample_near(start, repeated);
            }
        }
    }

    #[test]
    fn generated_phrases_fill_the_target_and_avoid_repeated_footwork() {
        for seed in 0..32 {
            let phrase = generate_phrase(seed, 256.0);
            assert_eq!(phrase.beats, 256.0);
            let events: Vec<_> = phrase
                .events
                .iter()
                .filter(|event| definition(event.id).layer == ActionLayer::Footwork)
                .collect();
            // The closing action is selected for loop recovery rather than variety.
            for pair in events[..events.len() - 1].windows(2) {
                assert_ne!(pair[0].id, pair[1].id);
                assert_eq!(pair[0].start_beat + pair[0].beats, pair[1].start_beat);
            }
            assert!(
                phrase.events.iter().all(
                    |event| event.beats > 0.0 && event.start_beat + event.beats <= phrase.beats
                )
            );
        }
    }
}
