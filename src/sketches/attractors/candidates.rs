//! Sketch-local candidate settings; sources and exploratory presets are documented.

use super::math::Equation;

pub(super) struct Candidate {
    pub(super) id: usize,
    pub(super) default_trajectories: usize,
    pub(super) name: &'static str,
    pub(super) equation: Equation,
    pub(super) step: f64,
    pub(super) steps_per_tick: usize,
    pub(super) warmup_steps: usize,
    pub(super) initial_center: [f64; 3],
    pub(super) initial_extent: f64,
    pub(super) view_center: [f32; 3],
    pub(super) framing_radius: f32,
}

const fn thomas(b: f64, radius: f32) -> Candidate {
    Candidate {
        id: 0,
        default_trajectories: 24,
        name: "Thomas",
        equation: Equation::Thomas { damping: b },
        step: 0.025,
        steps_per_tick: 2,
        warmup_steps: 20_000,
        initial_center: [0.0; 3],
        initial_extent: 1.0,
        view_center: [0.0; 3],
        framing_radius: radius,
    }
}

const fn rossler(a: f64, c: f64, radius: f32) -> Candidate {
    Candidate {
        id: 0,
        default_trajectories: 24,
        name: "Rössler",
        equation: Equation::Rossler { a, b: 0.2, c },
        step: 0.01,
        steps_per_tick: 3,
        warmup_steps: 30_000,
        initial_center: [0.1, 0.0, 0.1],
        initial_extent: 0.05,
        view_center: [0.0, 0.0, 8.0],
        framing_radius: radius,
    }
}

const fn aizawa(b: f64, c: f64) -> Candidate {
    Candidate {
        id: 0,
        default_trajectories: 24,
        name: "Aizawa",
        equation: Equation::Aizawa {
            parameters: [0.95, b, c, 3.5, 0.25, 0.1],
        },
        step: 0.005,
        steps_per_tick: 4,
        warmup_steps: 30_000,
        initial_center: [-0.7845, -0.6289, -0.1762],
        initial_extent: 0.02,
        view_center: [0.0, 0.0, 0.7],
        framing_radius: 2.0,
    }
}

const fn halvorsen(a: f64) -> Candidate {
    Candidate {
        id: 0,
        default_trajectories: 24,
        name: "Halvorsen",
        equation: Equation::Halvorsen { a },
        step: 0.0025,
        steps_per_tick: 4,
        warmup_steps: 30_000,
        initial_center: [1.0, 0.0, 0.0],
        initial_extent: 0.02,
        view_center: [-3.0; 3],
        framing_radius: 16.0,
    }
}

const fn dadras(c: f64) -> Candidate {
    Candidate {
        id: 0,
        default_trajectories: 24,
        name: "Dadras",
        equation: Equation::Dadras {
            parameters: [3.0, 2.7, c, 2.0, 9.0],
        },
        step: 0.0025,
        steps_per_tick: 4,
        warmup_steps: 30_000,
        initial_center: [1.0284, -3.5349, -0.7208],
        initial_extent: 0.02,
        view_center: [0.0; 3],
        framing_radius: if c < 3.0 { 37.0 } else { 21.0 },
    }
}

const fn with_density(mut candidate: Candidate, count: usize) -> Candidate {
    candidate.default_trajectories = count;
    candidate
}

const fn with_id(mut candidate: Candidate, id: usize) -> Candidate {
    candidate.id = id;
    candidate
}

// One representative per family, with stable IDs and saved density defaults.
pub(super) const CANDIDATES: &[Candidate] = &[
    with_id(thomas(0.208186, 8.32), 1),
    with_id(with_density(rossler(0.2, 5.7, 20.0), 15), 3),
    with_id(aizawa(0.7, 0.6), 4),
    with_id(halvorsen(1.27), 5),
    with_id(dadras(1.7), 6),
    Candidate {
        id: 7,
        default_trajectories: 24,
        name: "Rucklidge",
        equation: Equation::Rucklidge { a: 2.0, b: 6.7 },
        step: 0.005,
        steps_per_tick: 4,
        warmup_steps: 30_000,
        initial_center: [1.9257158, 3.1923835, 5.7829655],
        initial_extent: 0.02,
        view_center: [0.0, 0.0, 6.4],
        framing_radius: 16.0,
    },
    Candidate {
        id: 8,
        default_trajectories: 24,
        name: "Burke–Shaw",
        equation: Equation::BurkeShaw { e: 13.0, n: 10.0 },
        step: 0.001,
        steps_per_tick: 5,
        warmup_steps: 30_000,
        initial_center: [-0.79153188, 0.65244479, 0.22517831],
        initial_extent: 0.02,
        view_center: [0.0; 3],
        framing_radius: 9.0,
    },
    Candidate {
        id: 9,
        default_trajectories: 12,
        name: "Chen–Lee",
        equation: Equation::ChenLee {
            a: 5.0,
            b: -10.0,
            c: -0.38,
        },
        step: 0.0025,
        steps_per_tick: 4,
        warmup_steps: 40_000,
        initial_center: [0.17177112, 0.10360479, 6.2553878],
        initial_extent: 0.02,
        view_center: [0.0, 0.0, 8.2],
        framing_radius: 23.0,
    },
    Candidate {
        id: 10,
        default_trajectories: 24,
        name: "Rabinovich–Fabrikant",
        equation: Equation::RabinovichFabrikant { a: 1.1, g: 0.87 },
        step: 0.0025,
        steps_per_tick: 4,
        warmup_steps: 40_000,
        initial_center: [-1.8207276, 0.83440917, 0.2078248],
        initial_extent: 0.0001,
        view_center: [0.0, 0.0, 0.6],
        framing_radius: 3.3,
    },
    Candidate {
        id: 11,
        default_trajectories: 24,
        name: "Sprott B",
        equation: Equation::SprottB,
        step: 0.01,
        steps_per_tick: 3,
        warmup_steps: 30_000,
        initial_center: [1.6878282, 1.4848132, -0.53706628],
        initial_extent: 0.02,
        view_center: [0.0; 3],
        framing_radius: 9.0,
    },
];
