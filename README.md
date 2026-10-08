# Kaleidoodles

**Project status: paused / abandoned.** Active development has ended as the experiments reached diminishing returns; work is moving to a new project. The code and documentation remain available for reference, with no further work planned here.

A Rust and Bevy 0.19.1 creative-coding app with four sketches: a 3D attractor collection, a procedural stickman dancer, a fish ecosystem, and a particle flow field.

## Run

From the repository root, with stable Rust and a working graphics driver:

```sh
cargo run --release -j 4
```

The attractor preview shows its candidate and parameters on screen; use R to restart, Left/Right to switch ten candidates, one per family, and Up/Down to explore density. Original IDs 01 and 03–06 are retained; Lorenz (02) was removed as similar to Rucklidge; IDs 07–11 introduce new families. Keys 1–9 select directly, and Shift+1/2 select 10/11; use `--candidate <id>` for direct selection.

The attractor is selected by `ACTIVE_SKETCH` in `src/sketches.rs`; change that constant to select `Dancer`, `FishTank`, or `FlowField`. There is no runtime sketch selector. For the dancer, without `--bpm`, live sessions open the default microphone; tempo tracking is experimental. Manual BPM accepts values from 30 to 300. `--move-demo --bpm 139` runs the numbered 19-action walkthrough.

## Export a reel

Requires ffmpeg and a working GPU even though recording creates no window. Use a fresh frame directory and a new output filename for each capture:

```sh
cargo run --release -j 4 -- --record data/dancer/frames-139bpm-seed42/ --bpm 139 --seed 42 --fps 30 --duration 60
ffmpeg -nostdin -n -framerate 30 -i data/dancer/frames-139bpm-seed42/%04d.png \
  -frames:v 1800 -c:v libx264 -pix_fmt yuv420p -crf 18 -movflags +faststart \
  data/dancer/dance-139bpm-seed42.mp4
```

Select `Dancer` before using that example; the currently selected attractor can be captured with the same flags except `--bpm` and `--seed`, using paths under `data/attractors/`.

This captures 1,800 PNG frames and produces a silent 60-second, 30 fps, 1080 × 1920 video. Add music separately. Keep artifacts under `data/<sketch-name>/`, which is gitignored. After checking the video, remove only that capture's frame directory to reclaim space.

Recording defaults to 60 fps and 10 seconds. Simulation advances by `1/fps`; the saved frame count truncates `fps × duration` to an integer. Eight warm-up renders are discarded while simulation continues. A seed reproduces choreography, not necessarily identical pixels across runs. GPU readback and synchronous PNG encoding can make capture slower than playback; release builds avoid the debug PNG-encoding slowdown seen on the one-minute dancer capture. The configured fps describes the exported timeline, not guaranteed capture throughput.

## Fish simulation and sweeps

Select `FishTank` first. `cargo run --release -j 4 -- --simulate --fps 30 --duration 120` runs without PNG output or a microphone, but still initializes rendering. Simulation defaults are 30 fps and 120 seconds. `--demo` uses larger, slower fish and event rings; it starts with 92 fish.

Population observations occur every 150 simulation frames (five seconds at 30 fps). The final `RESULT` line is emitted through that observation path, so use a total frame count divisible by 150 when collecting sweep results. Reported minima and extinction observations are sampled rather than checked every frame; `stocked` records immigration events. The default calibration covers 180 seconds, and longer runs may depend on immigration.

`scripts/sweep.sh` builds the release app and fans out fish simulations; configure its parameter grids through environment variables such as `RATES`, `SEEDS`, `OUT`, or `RANDOM_RUNS`. Set `CARGO_BUILD_JOBS=4` when invoking the script to apply the build job limit. Its `OUT` directory is replaced at startup, so choose a new directory to preserve previous results. Sweep directories are gitignored. See the script for the complete parameter list.

## Code and documentation

`src/app.rs` assembles the app, `src/recording.rs` handles frame capture, and `src/sketches.rs` selects the sketch. Shared camera and timing helpers live in `src/sketches/common.rs`; dancer rendering, choreography, and audio live in `dancer.rs`, `dancer/moves.rs`, and `dancer/audio.rs` respectively.

- [Ledger](docs/Ledger.md): built art pieces.
- [Attractors](docs/Attractors.md): mathematical model and first candidate.
- [Dancer](docs/Dancer.md): current motion design and remaining personality work.
- [Future directions](docs/future.md): deferred experiments and microphone limitations.
- [Queue](docs/Queue.md): the sole project backlog.
- [Software Quality](docs/Software%20Quality.md) and [Audit History](docs/Audit%20History.md): methods, run log, and rerun triggers.
- [AGENTS.md](AGENTS.md): development, validation, and collaboration rules.

## Idea catalog

A nonbinding catalog of creative-coding possibilities; checked entries have been implemented in sketches or experiments. See [the ledger](docs/Ledger.md) for the built pieces.

## Generative Art

- [x] Particle systems
- [x] Flow fields
- [ ] Perlin noise
- [x] Procedural character animation
- [ ] Particle trails
- [ ] Attractors
- [ ] Random walks
- [ ] L-systems
- [ ] Fractals
- [ ] Recursive branching
- [ ] Space-filling curves
- [ ] Reaction-diffusion
- [ ] Procedural landscapes
- [ ] Terrain generation
- [ ] Contour lines
- [ ] Voronoi diagrams
- [ ] Delaunay triangulation
- [ ] Tessellation and tiling
- [ ] Penrose patterns
- [ ] Parametric architecture
- [ ] Generative typography
- [ ] Procedural textures
- [ ] Fractal flames
- [ ] Impossible geometry

## Artificial Life & Emergent Systems

- [x] Boids and flocking
- [x] Predator-prey simulation
- [x] Agent-based simulations
- [ ] Conway's Game of Life
- [ ] Elementary cellular automata
- [ ] Custom cellular automata
- [ ] Excitable media
- [ ] Lenia
- [ ] Continuous cellular automata
- [ ] Self-replicating patterns
- [ ] Digital evolution
- [ ] Ant colony simulation
- [ ] Pheromone trails
- [ ] Swarm intelligence
- [ ] Genetic algorithms
- [ ] Evolutionary art
- [ ] Neural cellular automata
- [x] Ecosystem simulation
- [ ] Self-organising systems
- [ ] Reaction networks

## Physics Simulations

- [ ] Fluid dynamics (Navier-Stokes)
- [ ] Smoke simulation
- [ ] Liquid simulation
- [ ] Vortices and turbulence
- [ ] Soft-body physics
- [ ] Elastic objects
- [ ] Cloth simulation
- [ ] Deformable meshes
- [ ] N-body gravitational simulation
- [ ] Orbital mechanics
- [ ] Particle collisions
- [ ] Spring systems
- [ ] Molecular dynamics
- [ ] Granular materials
- [ ] Wave propagation
- [ ] Optics and refraction
- [ ] Ray tracing
- [ ] Electromagnetic field visualisation
- [ ] Fire simulation
- [ ] Falling sand
- [ ] Fracture and destruction

## Interactive Visual Experiments

- [ ] Mouse-controlled fluid simulation
- [ ] Interactive particle systems
- [ ] Physics playground
- [ ] Generative drawing instrument
- [ ] Interactive fractals
- [ ] Parametric shape manipulation
- [ ] Gesture-controlled visuals
- [ ] 3D spatial exploration
- [ ] Interactive ecosystems
- [ ] Interactive shaders
- [ ] Real-time visual instruments

## Ray Marching & Signed Distance Fields

- [ ] Ray marching
- [ ] Signed distance functions
- [ ] Mandelbulb rendering
- [ ] Procedural 3D objects
- [ ] Fractal geometry
- [ ] Infinite tunnels
- [ ] Smooth shape blending
- [ ] SDF-based modelling

## Audio-Reactive Art

- [ ] FFT frequency analysis
- [ ] Bass-driven particles
- [ ] Midrange-driven geometry
- [ ] High-frequency visual effects
- [ ] Audio-reactive shaders
- [ ] Spectrogram visualisation
- [ ] Waveform visualisation
- [ ] Procedural music visualiser
- [ ] Audio-driven scene transitions
- [ ] Generative sound synthesis
- [ ] Audio-reactive 3D environments

## Mathematical & Scientific Visualisation

- [ ] Lorenz attractor
- [ ] Rössler attractor
- [ ] Mandelbrot set
- [ ] Julia sets
- [ ] Complex plane visualisation
- [ ] Domain colouring
- [ ] Differential equations
- [ ] Chaos theory
- [ ] Topology visualisation
- [ ] Hyperbolic geometry
- [ ] Fourier transforms
- [ ] Wave interference
- [ ] Probability distributions
- [ ] Monte Carlo simulations
- [ ] Higher-dimensional projections
- [ ] Numerical simulations

## Procedural Worlds & Environments

- [ ] Procedural planets
- [ ] Floating islands
- [ ] Infinite terrain
- [ ] Cave generation
- [ ] Procedural cities
- [ ] Fractal vegetation
- [ ] Ecosystem generation
- [ ] Weather systems
- [ ] Day/night cycles
- [ ] Procedural world generation
- [ ] Procedural architecture

## Shaders & Visual Effects

- [ ] GLSL fragment shaders
- [ ] Chromatic aberration
- [ ] Kaleidoscopic transformations
- [ ] Feedback loops
- [ ] Framebuffer feedback
- [ ] Domain warping
- [ ] Glitch effects
- [ ] Ray marching effects
- [ ] Procedural materials
- [ ] Metaballs
- [ ] Bloom and distortion
- [ ] Optical illusions
- [ ] Liquid surfaces
- [ ] Iridescent materials
- [ ] Psychedelic visual effects

## Reference

- [Shan Shui Inf](https://github.com/LingDong-/shan-shui-inf) is a source of inspiration for future procedural-landscape sketches.
