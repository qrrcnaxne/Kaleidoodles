# Ledger

## Purpose

Catalog the generative art pieces built in this repo.

## Brownian motion

**Status:** Implemented, visually accepted, and exported.

A finite two-dimensional gas of 220 small hard disks transfers momentum through elastic collisions to five larger tracers distributed across the container. Reflecting walls contain the bodies, and short color-matched fading trails reveal each tracer's motion. The sketch uses fixed-step simulation and supports repeatable reset and seeded variation.

See [Brownian motion](Brownian%20Motion.md) for the model, controls, and physical limits; implementation lives in [brownian_motion.rs](../src/sketches/brownian_motion.rs).

The silent export is `data/brownian-motion/five-tracers-seed42-60s-30fps.mp4`: 60 seconds at 30 fps and 1080 × 1920, seed 42. It contains 1,800 H.264/yuv420p frames and is 25.8 MB.

## Crystal growth

**Status:** Seeded 60-second CPU replay visually accepted and exported.

A bounded 180 × 320 lattice DLA sketch starts with one seed near the top. One hundred and twenty-eight unbiased random walkers enter below the cluster, diffuse through cardinal and diagonal steps, and attach on contact. The complete seeded run is precomputed and its genuine growth sequence is replayed evenly over 60 seconds, favoring downward growth without prescribing a branch shape. Lattice anisotropy makes this branching aggregation rather than a faceted-crystal model. R restarts the current deterministic seed, N advances to a new seed, and Space pauses.

See [Crystal growth](Crystal%20Growth.md) for the model and limitations; implementation lives in [crystal_growth.rs](../src/sketches/crystal_growth.rs).

The silent export is `data/crystal-growth/top-down-dla-seed42-complete-60s-30fps.mp4`: 60 seconds at 30 fps and 1080 × 1920, seed 42, with 1,800 H.264/yuv420p frames; file size is 8.4 MB.

## Reaction–diffusion

**Status:** CPU artwork visually accepted and exported.

Two chemical concentrations evolve through the driven Gray–Scott equations on a 180 × 320 periodic grid, rendered as a portrait neon texture. The feed/kill pair is selected from MIT's dynamic “wandering bubbles” example; deterministic seed patches and an 800-step warm-up establish patterns before playback. Chemical concentration drives the palette, which also drifts over time. R restarts and Space pauses. See [Reaction–diffusion](Reaction%20Diffusion.md) for mathematics and settings.

The silent export is `data/reaction-diffusion/wandering-bubbles-200steps-60s-30fps.mp4`: 60 seconds at 30 fps and 1080 × 1920, rendered at 200 simulation steps per second. It contains 1,800 H.264/yuv420p frames and is 37.2 MB.

## 3D attractor collection

**Status:** Ten candidates, one per family, implemented, numerically checked, accepted visually, and exported.

Thomas, Rössler, Aizawa, Halvorsen, Dadras, Rucklidge, Burke–Shaw, Chen–Lee, Rabinovich–Fabrikant, and Sprott B flows produce spatial rainbow trails, using candidate-specific counts of deterministic trajectories with 1,350 points each. Three-axis rotation (60/30/15-second X/Y/Z turns) and perspective reveal their depth. Fixed-step RK4 integrates family-specific equations; numbered controls allow direct comparison of ten family representatives. The initial trails are populated, framing adapts to the viewport, and density can be explored interactively.

The full collection is exported under `data/attractors/`: ten silent MP4s, each 30 seconds at 30 fps and 1080 × 1920, using release builds and saved densities (Rössler 15, Chen–Lee 12, all others 24 trajectories). `collection-30s-30fps.json` records file metadata; each video was verified to contain 900 frames, and temporary capture frames were removed.

See [Attractors](Attractors.md) for equations, sources, presets, and numerical limitations; implementation lives in [attractors.rs](../src/sketches/attractors.rs).

## Stickman dancer

**Status:** Incomplete and paused while development moves to another piece; we plan to return to it in the future. Remaining work is documented in [Dancer](Dancer.md#v4-personality-and-presentation) and [Future directions](future.md#incomplete-live-microphone-tempo).

A side-view stickman performs a seed-generated 256-beat dance assembled from 19 footwork, accent, and pose actions over a continuous groove. Rainbow limbs, white joints, fading endpoint trails, a perspective stage, and a sweeping disco globe provide the presentation. Kinematic foot targets and inverse kinematics drive the rig; horizontal silhouette fitting keeps travel inside portrait margins. A numbered catalog walkthrough is also available. Manual BPM supports repeatable reel capture; live microphone tempo remains experimental.

The current export is `data/dancer/rainbow-moonwalk-139bpm-seed42.mp4`: 60 seconds at 30 fps, 1080 × 1920, seed 42, and 139 BPM, without audio. See [Dancer](Dancer.md) for motion design and [the renderer](../src/sketches/dancer.rs) for implementation.

## Fish tank

A CPU-boid ecosystem with four trophic levels: cyan fish eat sinking pellets, and each larger species hunts the one below it. Meals fund reproduction; starvation, age, and prey escapes limit populations. Schooling, fleeing, darting, repulsion, and wall avoidance shape the motion. Immigration can restock depleted species every five simulated seconds but does not prevent temporary extinction. Default parameters were calibrated over 180 seconds; the demo enlarges and slows 92 founder fish and marks births, kills, and natural deaths with rings.

Implementation: [fish_tank.rs](../src/sketches/fish_tank.rs).

## Flow field

A 2D sketch of 1,600 particles moving through a sinusoidal vortex flow field, with drifting hues. Particles start in a deterministic Halton-sequence distribution and wrap at viewport edges.

Implementation: [flow_field.rs](../src/sketches/flow_field.rs).
