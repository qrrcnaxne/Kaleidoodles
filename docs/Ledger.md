# Ledger

## Purpose

Catalog the generative art pieces built in this repo.

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
