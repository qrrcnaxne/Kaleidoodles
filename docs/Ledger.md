# Ledger

## Purpose

Catalog the generative art pieces built in this repo—the sketches themselves, not infrastructure or project/process documentation.

## Fish tank (`src/sketches/fish_tank.rs`)

A 2D CPU-boid ecosystem with four fish trophic levels: cyan fish eat sinking pellets, and each larger level hunts only the one directly below it. Fish bank meal energy to reproduce; half of the reproductive investment transfers to one offspring, while starvation, lifespan, and probabilistic prey escapes limit populations. Pellets spawn throughout the tank; species differ in size, speed, schooling behavior, energy use, and hunting success. Prey flee and dart from nearby predators, all fish repel one another at close range, and fish steer away from and bounce off tank walls. Immigration insurance stocks an adult into any species that drops below a small population floor, so no level can die out. Its default parameters are a calibrated equilibrium, found by randomized search, in which all four levels stay clear of that floor for a full 180-second reel. A `--demo` profile presents the same rules at reading pace instead: a handful of large, slow fish with expanding rings marking each birth, kill, and natural death.

## Flow field (`src/sketches/flow_field.rs`)

A 2D generative sketch of 1,600 particles moving through a sinusoidal vortex flow field, with hues slowly drifting across green, blue, violet, and pink. Particles start in a deterministic Halton-sequence distribution and wrap at the viewport edges.
