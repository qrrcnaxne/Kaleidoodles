# Implementation Ledger

A running list of what exists in Kaleidoodles so we don't duplicate work.

## Project shape

- Rust binary app using **Bevy 0.19.1** (edition 2024).
- Single crate, no workspace yet.
- Module layout follows `name.rs` beside `name/`:
  - `src/main.rs` — entry point, delegates to `app::run()`.
  - `src/app.rs` — wires plugins and handles `--record` mode.
  - `src/core.rs` — `CorePlugin` (currently a placeholder for shared camera/input/RNG/debug utilities).
  - `src/sketches.rs` — registers the active sketch plugin.
  - `src/sketches/flow_field.rs` — first sketch.
  - `src/recording.rs` — deterministic frame-capture infrastructure.

## Implemented sketches

### Flow field (`src/sketches/flow_field.rs`)

- 1,600 CPU-simulated particles on a sinusoidal vortex flow field.
- Shared cyan circle mesh/material for batch-friendly rendering.
- Deterministic Halton-sequence initial particle distribution (no RNG dependency).
- Viewport wrapping and `Time`-driven motion.
- Two unit tests: flow-direction unit length and coordinate wrapping.

## Recording / video export

- CLI flag `--record <directory>` triggers deterministic capture.
- Records 600 frames at a fixed 60 fps timestep for a 10-second clip.
- Target output resolution: **1080 × 1920** (9:16 Instagram Reels).
- Artifacts live outside git:
  - `frames/` — per-frame PNG sequence.
  - `data/` — final encoded `.mp4` files.
- Renders to a strict 1080 × 1920 offscreen `Image` render target so the captured PNGs are exactly 1080 × 1920, independent of the display or window-manager constraints.
- Keeps a small primary window open to satisfy Winit/WGPU requirements.

## Documentation

- `AGENTS.md` — project rules, Rust validation commands, and the video-export workflow.
- `docs/Queue.md` — one-sentence queue of upcoming work.
- `docs/pi-research.md` — research notes on Pi capabilities and model roles.
- `docs/pi-ideas.md` — early idea scratchpad.
- `docs/Ledger.md` — this file.

## Tooling / conventions

- Validation: `cargo fmt`, `cargo check -j 4 --all-targets --all-features`, `cargo clippy -j 4 --all-targets --all-features -- -D warnings`, `cargo test -j 4 --all-targets --all-features`.
- Dependencies: `bevy`, `crossbeam-channel`, `image`.
