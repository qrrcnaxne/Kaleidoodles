# Crystal growth

## Purpose

Describe the diffusion-limited aggregation sketch, its lattice model, preview behavior, and physical limits.

## Model

The sketch is a bounded, fed diffusion-limited aggregation (DLA) model inspired by Witten and Sander's original lattice study, [“Diffusion-Limited Aggregation, a Kinetic Critical Phenomenon”](https://journals.aps.org/prl/abstract/10.1103/PhysRevLett.47.1400) (Physical Review Letters 47, 1400, 1981).

A single occupied site near the top starts the cluster. One hundred and twenty-eight walkers are injected below its current lowest point, perform unbiased random walks on a 180 × 320 lattice, and stick when a step places them next to any occupied site. Walkers that reach a wall are recycled at the source band. This bounded source-fed setup favors downward growth because the supply enters from below; it does not impose branch directions or a target silhouette. The cardinal-and-diagonal lattice and eight-site contact rule introduce lattice anisotropy, so the result is branching aggregation rather than a model of faceted mineral crystals.

## Presentation and controls

Before playback, the sketch runs the seeded DLA process to the lower boundary and stores its actual attachment order and walker positions. The preview replays that generated history across 60 seconds, reaching completion just before the end and holding the complete cluster briefly. This is a deterministic time-warped replay, not a claim that the accelerated precomputation is physical real-time growth. Generation is capped at 100,000 ticks; if a seed does not reach the target within that bound, the sketch displays an explicit failure instead of presenting a partial cluster as complete. Seed 42 is the default, and N advances to the next deterministic seed.

The preview maps the lattice to a 540 × 960 portrait view. A muted blue cluster grows downward from an amber seed while pale dots replay the recorded random walkers. Generation uses 16 random lattice steps per walker per tick. During recording, playback spans the saved-frame count minus an eight-frame tail, preserving the seed at the start and allowing a brief completed hold for render-readback and entity-update latency; the recording warm-up does not consume the minute. Use `--fps 30 --duration 60` for a 60-second, 30 fps capture.

- R restarts the current seed's deterministic run.
- N restarts with the next seed.
- Space pauses and resumes the simulation.

The seed-42 replay was visually accepted and exported; the completed video is cataloged in the [ledger](Ledger.md#crystal-growth). This small CPU lattice uses individual mesh entities for visible sites rather than adding another texture renderer or generalized simulation infrastructure.

Implementation: [crystal_growth.rs](../src/sketches/crystal_growth.rs).
