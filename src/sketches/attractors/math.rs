//! Sketch-local equations and fourth-order Runge–Kutta integration.

#[derive(Clone, Copy)]
pub(super) enum Equation {
    Thomas { damping: f64 },
    Rossler { a: f64, b: f64, c: f64 },
    Aizawa { parameters: [f64; 6] },
    Halvorsen { a: f64 },
    Dadras { parameters: [f64; 5] },
    Rucklidge { a: f64, b: f64 },
    BurkeShaw { e: f64, n: f64 },
    ChenLee { a: f64, b: f64, c: f64 },
    RabinovichFabrikant { a: f64, g: f64 },
    SprottB,
}

impl Equation {
    fn derivative(self, [x, y, z]: [f64; 3]) -> [f64; 3] {
        match self {
            Self::Thomas { damping } => [
                y.sin() - damping * x,
                z.sin() - damping * y,
                x.sin() - damping * z,
            ],
            Self::Rossler { a, b, c } => [-y - z, x + a * y, b + z * (x - c)],
            Self::Aizawa {
                parameters: [a, b, c, d, e, f],
            } => [
                (z - b) * x - d * y,
                d * x + (z - b) * y,
                c + a * z - z.powi(3) / 3.0 - (x * x + y * y) * (1.0 + e * z) + f * z * x.powi(3),
            ],
            Self::Halvorsen { a } => [
                -a * x - 4.0 * y - 4.0 * z - y * y,
                -a * y - 4.0 * z - 4.0 * x - z * z,
                -a * z - 4.0 * x - 4.0 * y - x * x,
            ],
            Self::Dadras {
                parameters: [a, b, c, d, e],
            } => [y - a * x + b * y * z, c * y - x * z + z, d * x * y - e * z],
            Self::Rucklidge { a, b } => [-a * x + b * y - y * z, x, -z + y * y],
            Self::BurkeShaw { e, n } => [-n * (x + y), y - n * x * z, n * x * y + e],
            Self::ChenLee { a, b, c } => [a * x - y * z, b * y + x * z, c * z + x * y / 3.0],
            Self::RabinovichFabrikant { a, g } => [
                y * (z - 1.0 + x * x) + g * x,
                x * (3.0 * z + 1.0 - x * x) + g * y,
                -2.0 * z * (a + x * y),
            ],
            Self::SprottB => [y * z, x - y, 1.0 - x * y],
        }
    }

    pub(super) fn parameters(self) -> String {
        match self {
            Self::Thomas { damping } => format!("b={damping}"),
            Self::Rossler { a, b, c } => format!("a={a} b={b} c={c}"),
            Self::Aizawa {
                parameters: [a, b, c, d, e, f],
            } => format!("a={a} b={b} c={c} d={d} e={e} f={f}"),
            Self::Halvorsen { a } => format!("a={a}; coupling=4"),
            Self::Dadras {
                parameters: [a, b, c, d, e],
            } => format!("a={a} b={b} c={c} d={d} e={e}"),
            Self::Rucklidge { a, b } => format!("a={a} b={b}"),
            Self::BurkeShaw { e, n } => format!("e={e} n={n}"),
            Self::ChenLee { a, b, c } => format!("a={a} b={b} c={c}"),
            Self::RabinovichFabrikant { a, g } => format!("a={a} γ={g}"),
            Self::SprottB => "fixed coefficients".to_string(),
        }
    }
}

fn offset(point: [f64; 3], delta: [f64; 3], scale: f64) -> [f64; 3] {
    std::array::from_fn(|i| point[i] + delta[i] * scale)
}

pub(super) fn advance(point: [f64; 3], equation: Equation, step: f64) -> [f64; 3] {
    let a = equation.derivative(point);
    let b = equation.derivative(offset(point, a, step * 0.5));
    let c = equation.derivative(offset(point, b, step * 0.5));
    let d = equation.derivative(offset(point, c, step));
    std::array::from_fn(|i| point[i] + step / 6.0 * (a[i] + 2.0 * b[i] + 2.0 * c[i] + d[i]))
}
