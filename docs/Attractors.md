# Strange attractors

## Purpose

Describe the mathematics, presets, and presentation of the 3D attractor collection.

## Collection

Ten candidates are implemented, one per genuine 3D family. Original representatives retain IDs 01 and 03–06; new families use IDs 07–11. Retired parameter variants are not restored: those numbers now refer to new equations. Lorenz (02) was discarded as visually similar to Rucklidge (07). Rössler (03) uses 15 trajectories, Chen–Lee (09) uses 12, and every other family uses 24, with 1,350 points per trail.

New candidates must introduce another family rather than another parameter set of an existing family. Numerical preflight passes for all ten; The user accepted the remaining families visually after removing 02; further similarity review applies to future additions. Bounded numerical trajectories alone do not prove chaos or exclude periodic motion.

## Equations and sources

The following right-hand sides define each autonomous flow; RK4 integrates them with the preset timestep.

| Family | dx/dt | dy/dt | dz/dt |
|---|---|---|---|
| Thomas | sin(y) − b*x | sin(z) − b*y | sin(x) − b*z |
| Rössler | −y−z | x+a*y | b+z*(x−c) |
| Aizawa | (z−b)*x−d*y | d*x+(z−b)*y | c+a*z−z³/3−(x²+y²)*(1+e*z)+f*z*x³ |
| Halvorsen | −a*x−4*y−4*z−y² | −a*y−4*z−4*x−z² | −a*z−4*x−4*y−x² |
| Dadras | y−a*x+b*y*z | c*y−x*z+z | d*x*y−e*z |
| Rucklidge | −a*x+b*y−y*z | x | −z+y² |
| Burke–Shaw | −n*(x+y) | y−n*x*z | n*x*y+e |
| Chen–Lee | a*x−y*z | b*y+x*z | c*z+x*y/3 |
| Rabinovich–Fabrikant | y*(z−1+x²)+γ*x | x*(3*z+1−x²)+γ*y | −2*z*(a+x*y) |
| Sprott B | y*z | x−y | 1−x*y |

Sources checked for this implementation:

- [Sprott and Chlouverakis, Labyrinth Chaos](https://sprott.physics.wisc.edu/pubs/paper302.pdf): Thomas flow.
- [GilpinLab dysts equations](https://github.com/GilpinLab/dysts/blob/master/dysts/flows.py) and [reference metadata](https://github.com/GilpinLab/dysts/blob/master/dysts/data/chaotic_attractors.json): cross-checks, Aizawa/Dadras reference parameters, and equations, reference parameters, and initial centers for all five new families; Aizawa is the library's naming convention, whose metadata also cites Langford's torus-bifurcation work.
- [Bourke's Rössler description](https://paulbourke.net/fractals/rossler/): standard Rössler equations and parameters.
- [Sprott's symmetric chaotic flow](https://sprott.physics.wisc.edu/chaos/symmetry.htm): Halvorsen equations and a=1.27 reference.
- [Dadras–Momeni numerical study](https://www.mdpi.com/2227-7390/9/9/950): equations and c=4.7 parameter set.

These are mathematical dynamical systems; the renderer does not reconstruct physical apparatus.

## Presets

Equation parameters follow the references above; starting conditions and framing are local presentation choices.

| Number | Family | Parameters | Default trajectories |
|---|---|---|---|
| 01 | Thomas | b=0.208186 | 24 |
| 03 | Rössler | a=0.2, b=0.2, c=5.7 | 15 |
| 04 | Aizawa | a=0.95, b=0.7, c=0.6, d=3.5, e=0.25, f=0.1 | 24 |
| 05 | Halvorsen | a=1.27, coupling=4 | 24 |
| 06 | Dadras | a=3, b=2.7, c=1.7, d=2, e=9 | 24 |
| 07 | Rucklidge | a=2, b=6.7 | 24 |
| 08 | Burke–Shaw | e=13, n=10 | 24 |
| 09 | Chen–Lee | a=5, b=−10, c=−0.38 | 12 |
| 10 | Rabinovich–Fabrikant | a=1.1, γ=0.87 | 24 |
| 11 | Sprott B | fixed coefficients | 24 |

## Integration and framing

Starting points are deterministic Halton samples around each preset's initial center. Warm-up removes the initial transient before filling the trails. Each 1/60-second animation tick advances the family-specific number of mathematical steps; live catch-up is capped after a window stall, while headless runs use fixed simulation timing.

| Family | RK4 step | Steps per tick | Simulation units per playback second | Warm-up steps |
|---|---|---|---|---|
| Thomas | 0.025 | 2 | 3.0 | 20,000 |
| Rössler | 0.01 | 3 | 1.8 | 30,000 |
| Aizawa | 0.005 | 4 | 1.2 | 30,000 |
| Halvorsen | 0.0025 | 4 | 0.6 | 30,000 |
| Dadras | 0.0025 | 4 | 0.6 | 30,000 |
| Rucklidge | 0.005 | 4 | 1.2 | 30,000 |
| Burke–Shaw | 0.001 | 5 | 0.3 | 30,000 |
| Chen–Lee | 0.0025 | 4 | 0.6 | 40,000 |
| Rabinovich–Fabrikant | 0.0025 | 4 | 0.6 | 40,000 |
| Sprott B | 0.01 | 3 | 1.8 | 30,000 |

View centering is applied only to rendered coordinates, leaving the equations unchanged. The perspective camera fits a preset sphere through every rotation. Thomas radii enclose its invariant cube; other preset radii were checked over sampled numerical runs, not proved invariant. Framing expands if a later point exceeds the preset sphere and resets when the candidate restarts. The bounds check covers baseline seeds; unlimited density can introduce further initial points.

The full XYZ geometry rotates with X completing one turn in 60 seconds, Y in 30 seconds, and Z in 15 seconds (`Rz * Ry * Rx`). Orientation repeats every minute; evolving trajectories and color do not necessarily form a seamless loop. Bevy gizmos render fading rainbow line segments, with no new dependencies.

## Preview setup

`candidates.rs` holds presets, `math.rs` holds equations and RK4, and `attractors.rs` owns the preview, controls, numerical preflight, and rendering. The existing recording pipeline is reused without a general sketch registry.

Run `cargo run --release -j 4`, or select a preset directly with `cargo run --release -j 4 -- --candidate 7`; accepted IDs are 01 and 03–11; invalid values exit with an error.

| Control | Action |
|---|---|
| Left / Right | Previous or next candidate, wrapping through all ten candidates. |
| 1–9 | Select available IDs 01 and 03–09; 02 does nothing. |
| Shift + 1 / 2 | Select IDs 10 / 11. |
| R | Restart the current candidate deterministically at its default density. |
| Up / Down | Increase or reduce density. |
| Backspace | Restore the candidate default density without restarting motion. |

The live title and readout show candidate number, family, parameters, density, and controls. Switching or pressing R regenerates deterministic trajectories, prepopulates trails, restores its default density, and resets integration accumulation, hue time, rotation time, and framing. The first update after reset shows the starting state. Headless runs omit labels and keyboard controls but honor `--candidate`.

## Density exploration

The original comparison baseline is **24 trajectories × 1,350 points** (32,376 line segments). The selected candidate's trajectory default is listed in the preset table above.

Up adds roughly 50% more trajectories per press without a configured ceiling; Down reduces the count by roughly one third, down to one trajectory; Backspace restores the selected candidate default. Memory and rendering throughput set the practical limit. Extra trajectories are prepopulated; reducing density keeps the lowest-numbered trajectories, while re-added trajectories restart deterministically. Headless runs use the selected candidate's default density.

The user explored roughly 2,000 Thomas trajectories and found little additional visual structure, so defaults stay modest and can vary by candidate. Trail length, width, equations, and rotation remain fixed during density comparisons.

## Numerical preflight

`cargo run --release -j 4 -- --validate-attractors` runs without a GPU, window, microphone, or frame output. It uses the production initializer and RK4 equations for all ten representatives and at least 24 seeds per candidate (or its default count if larger), includes populated trails, and advances another 240 seconds of playback. It reports finite/bounded behavior, maximum distance from the view center, trajectory extent, and error against two half-sized RK4 steps over a short interval.

All retained presets passed with the configured framing radii, extent above 0.1, and short-interval coordinate error below 0.001; the largest measured error for the retained candidates was about 0.000004. These checks are finite numerical evidence, not a proof of chaos, global stability, long-time coordinate accuracy, or visual distinctness. Existing unit tests do not cover the new equations or controls. The user performs interactive visual review.

## Family implementation

All ten representatives are implemented and numerically checked. Parameter variants were removed because the user found the family structures too similar; retain one candidate per family going forward.

## Collection expansion

The first five new families are implemented as IDs 07–11. Ten families now remain; another eight would reach 18 clips and another ten would reach the user's 20-clip ceiling. Further additions require new equations, sourced settings, numerical preflight, and visual review; different equations do not guarantee different-looking results. No further family selection is settled yet.

## Collection review

Browse numbered candidates in one preview window, identify repeats or weak motion, and keep all distinct results the user wants. Tune their palette, playback, and framing after this comparison. Record review decisions here; pending work stays only in [Queue](Queue.md).

## Approved export stage

The user approved the full ten-family collection at 30 seconds each, 30 fps, portrait 1080 × 1920, using release builds and saved candidate defaults. Each clip contains 900 frames without audio. The shared rotation remains X/Y/Z turns in 60/30/15 seconds, so these clips cover half the slowest rotation cycle. Future exports require fresh explicit approval before recording frames or encoding video.

After approval, capture a selected candidate with `cargo run --release -j 4 -- --candidate 6 --record data/attractors/frames-dadras06/ --fps 30 --duration 30`, then use the [README encoding workflow](../README.md#export-a-reel) with attractor paths and 900 frames. BPM and seed flags are unused by this sketch. Store artifacts under `data/attractors/` and check encoded duration, dimensions, frame rate, and frame count. Collection filenames use `<ID>-<family>-30s-30fps.mp4`; `collection-30s-30fps.json` records saved densities and verified file metadata.
