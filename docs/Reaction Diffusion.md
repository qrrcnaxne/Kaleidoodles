# Reaction–diffusion

## Purpose

Describe the first Gray–Scott artwork, its numerical model, preview behavior, and accepted export.

## Model

Two dimensionless concentrations U and V diffuse and react:

```text
∂U/∂t = Du ∇²U − UV² + F(1 − U)
∂V/∂t = Dv ∇²V + UV² − (F + k)V
```

U feeds the reaction; V catalyzes conversion of U into more V and is removed over time. The [MIT Gray–Scott reference](https://groups.csail.mit.edu/mac/projects/amorphous/GrayScott/) describes the reaction model; [Stanford lecture notes](https://web.stanford.edu/class/cs279/lectures/old%20materials/lecture10.pdf) explain the update equations. This is an illustrative mathematical simulation, not a calibrated physical experiment.

The driven Gray–Scott setting uses Du=1, Dv=0.5, F=0.012, and k=0.05. The [MIT Gray–Scott reference](https://groups.csail.mit.edu/mac/projects/amorphous/GrayScott/) describes that feed/kill pair as a dynamic “wandering bubbles” pattern, with bubbles repeatedly colliding and regenerating. It is an externally driven model because U is continually fed and both U and V are removed; the concentrations are not conserved. This implementation is not a reproduction of MIT's exact grid or update scheme, so its appearance can differ. A 180 × 320 periodic grid uses a nine-point Laplacian (center −1, axial neighbors 0.2, diagonals 0.05), double-buffered forward Euler with dt=1, and concentration clipping to [0,1]. The parameter behavior depends on this stencil and timestep.

## Presentation and controls

The live window starts at 540 × 960. A texture displays the field with concentration-dependent neon colors and slow palette drift, scaled with preserved 9:16 aspect ratio; other window shapes show margins. The numerical grid remains 180 × 320 regardless of display or recording resolution, so this is a coarse preview rather than full-resolution chemical detail.

48 deterministic circular patches initialize U=0.5, V=0.25 within a background U=1, V=0. An 800-step warm-up establishes visible structure. Playback advances 200 numerical steps per simulated second using an accumulator; live frame delays are capped at 0.1 seconds to bound catch-up. Recording uses the existing fixed timestep. A prior standalone numerical preflight at 480 steps per second mirrored the production update with this grid, seed layout, and warm-up, and found mean absolute V-concentration changes of 0.075–0.086 per cell over each 10-second interval through simulated second 60. That historical check does not verify rendered appearance or activity at the current playback rate; pattern motion and palette animation are separate, so moving colors alone do not establish chemical evolution.

- R resets seeds, warm-up, simulation accumulator, palette clock, and pause state.
- Space pauses both chemistry and palette.

Only one setting is implemented. The visual result is accepted; longer-term pattern behavior and release-preview performance remain to be reviewed. A GPU simulation would require an architecture review.

## Export

The accepted silent export is `data/reaction-diffusion/wandering-bubbles-200steps-60s-30fps.mp4`, captured through a release build at 200 simulation steps per second, 30 fps, and 1080 × 1920. It contains 1,800 H.264/yuv420p frames over 60 seconds and is 37.2 MB. The encoded file passed a full decode check; temporary capture frames were removed after verification.

Implementation: [reaction_diffusion.rs](../src/sketches/reaction_diffusion.rs).
