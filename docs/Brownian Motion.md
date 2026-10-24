# Brownian motion

## Purpose

Describe the gas and tracer model, preview controls, and limits of the Brownian-motion sketch.

## Preview

The active CPU sketch places 220 radius-3, mass-1 molecules moving initially at 180–250 units per second and five radius-14, mass-16 tracers across a portrait container. Molecules begin with deterministic random positions and directions; tracers start at seeded, spread-out positions with zero velocity. Molecules and tracers collide elastically with one another. Reflecting walls reverse the impacted velocity component, and overlap correction separates bodies according to their inverse masses.

The simulation uses 1/240-second physics steps. Live frame time is capped at 0.1 seconds before entering the accumulator; recording uses the existing fixed recording timestep. The container is 540 × 960 simulation units with an 18-unit wall margin and scales uniformly to fit the viewport. Each tracer has a 3-second, low-contrast trail in its own color.

The seed defaults to 42 and can be set with `--seed <number>`. R restores the same initial state, N advances to the next seed, M hides or shows the molecules, and Space pauses or resumes.

## Physical limits

This is a finite two-dimensional hard-disk gas illustration. Equal-mass molecules and heavier tracers exchange momentum through pairwise elastic impulses, so the tracers' irregular motion emerges from discrete collisions. The sketch does not model a three-dimensional fluid, solvent molecules, molecular-scale time or length, or measured diffusion coefficients. Finite particle count and initial conditions affect their paths.

Implementation: [brownian_motion.rs](../src/sketches/brownian_motion.rs).
