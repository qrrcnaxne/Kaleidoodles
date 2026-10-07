# Procedural dancer

## Purpose

Describe the current dancer motion model, presentation, and remaining design work.

## Motion and presentation

The dancer uses a kinematic side-view rig: body angles and offsets combine with foot targets, and two-link inverse kinematics solves the legs. This is the current visual baseline; a possible physics treatment is documented in [Future directions](future.md#real-physics-experiment).

Manual BPM or experimental live microphone tempo drives a shared beat clock, which samples choreography into a `MotionSample` and then into rendered joints and synchronized disco effects. Tempo input is independent of choreography.

The stickman has white joint markers, opposing rainbow limb colors, and full-spectrum torso and head outlines. Eight endpoint trails fade over 0.75 seconds. A trapezoidal stage and a faceted silver globe frame the dancer; rainbow beams sweep with the phrase and flash in left, right, and center groups every four beats. The same clock drives motion and lights.

The leg solver and lean are artistic approximations, without a physics or balance solver. Foot contact targets guide planted, swinging, and sliding states; they do not establish physical support forces.

## Microphone-driven tempo

Live sessions can open the operating system's default microphone through CPAL. Its callback passes samples through a fixed-capacity, lock-free ring buffer to a worker that estimates tempo from short RMS windows; an adaptive volume gate and recent-BPM fallback are also implemented. The live tracker remains experimental and incomplete because real music has produced slow, incorrect locks; see [Future directions](future.md#incomplete-live-microphone-tempo). The `--bpm <value>` option is the reliable manual tempo source for reel work: it overrides microphone startup and works in live previews and headless captures. Headless runs without `--bpm` remain still. Only short-window RMS levels are analyzed; raw microphone samples are not saved.

**Acceptance:** in a live session, the dancer and lights follow a stable music tempo without disruptive timing jumps; temporary beat ambiguity while audio remains present uses the recent confirmed BPM mean, while sustained volume loss returns the dancer to neutral and stops the rays.

## Layered dance vocabulary

The vocabulary and deterministic sampler live in `src/sketches/dancer/moves.rs` and use no Bevy types. The composer combines a continuous groove, one active footwork event, and overlapping accents and poses into a `MotionSample`.

The catalog has 19 actions. `--move-demo` plays them in order with a flashing number, action name, authored duration, and one-second neutral gap. Default playback uses the generated phrase described below. The user has accepted its choreography for the reel.

### Rendering semantics

Turns project hips, knees, ankles, toes, and upper-body joints through the same side-view facing transform. Lean approximates an ankle pivot through pelvis displacement and upper-body tilt. Foot contact parts (flat, heel, ball, toe) affect geometry; weight distribution remains sampler data rather than a balance solution. Accent envelopes peak at onset so hits land on their authored beat.

### Layers

- **Groove:** a low-amplitude beat-driven weight shift and knee bounce. It is a background mode, not an action in the catalog; a freeze can mute it temporarily.
- **Footwork:** step-touch, shuffle, stomp, heel-toe, cross-step, backstep, knee-lift, kick-step, jump, slide, pivot, and moonwalk. These own foot contact, root travel, and turn progress.
- **Accents:** chest pop, shoulder hit, and head snap. These are short pulse envelopes that add only their body channels and can coincide with footwork.
- **Poses:** freeze, crouch, lean, and toe stand. These are eased overlays; toe stand adjusts ankle/toe geometry so the toes remain grounded as the ankles rise.

### Data model and sampling

- `ActionDefinition` describes an `ActionId`, its layer and composition tags, and either footwork keyframes or a pulse/held pose target.
- `ActionInstance` places an action at a beat offset, gives it a variable beat duration, selects a side, and sets intensity and travel scales. The phrase is an event list rather than one move per musical sentence; the authored reference composition uses 0.8-, 1-, 2-, 4-, and 8-beat events.
- `PoseDelta` adds selected joint channels over the groove and active footwork. Chest extension and independent shoulder lift affect the rig, while the head-snap channel rotates the facial cue in radians without sliding the head or stretching the neck.
- `ActionKeyframe` stores normalized phase, additive body targets, root/turn offsets, easing, and per-foot `Plant`, `Swing`, or `Slide` intent. Side mirroring swaps feet and left/right body channels.
- For authored playback, the stateless sampler accumulates completed footwork travel and unwrapped turn angle, then overlays active accents and poses. The 64-beat phrase closes its +48/-48-unit slide/moonwalk travel while retaining one full pivot turn; at 145 BPM, it takes about 26 seconds per cycle.

### Add-a-vocabulary-item recipe

1. Choose the layer first: footwork owns feet/root, an accent owns a brief body pulse, or a pose owns an eased held target.
2. Add a data definition and tags; use keyframes for footwork or a `PoseDelta` envelope for accents/poses.
3. Add one or more timed `ActionInstance`s to the phrase, choosing a beat offset, duration, side, intensity, and travel scale; overlap only independent layers.
4. Add a sampler test for timing, contact, mirroring, and overlap. Existing rig channels should be reused before proposing new renderer controls.

The authored phrase demonstrates sub-beat accents over footwork; generated phrases select accents on whole beats. Tests cover generation, move boundaries, and loop closure. Vocabulary changes should retain distinct moves, legible overlaps, and supported toe-stand geometry.

## Seeded procedural composition

Default playback generates exactly 256 beats from `--seed <u64>` (default `0`); at 145 BPM, one loop takes about 106 seconds. Before the closing passage, the generator avoids immediate footwork repeats, chooses sides deterministically, and adds accents and held poses. The final four beats provide a deliberate closing slide back to the starting stance. The same seed reproduces the event list; microphone timing does not reproduce a fixed performance.

Each footwork definition provides an `exit_phase` before its demo recovery. Generated playback samples only through that exit, carries foot anchors and accumulated turn into the next action, and blends the preceding exit pose, root, and foot lift into the new action over half a beat. Held keyframe jumps are eased for generated playback. The walkthrough keeps the complete authored clips and its neutral gaps.

Generated loops close their root and feet without accumulating travel, and carry turns as complete revolutions. Accents are omitted from the closing passage and the opening beat so the loop boundary has no accent reset. Pose and foot continuity are covered by tests across several seeds. Rendering constrains the complete silhouette to the viewport with six-percent horizontal margins, translating the whole rig together when accumulated travel reaches an edge. An unusually wide stance is uniformly reduced around floor height if translation alone cannot fit it. This applies to live previews and the 1080 × 1920 recording target; endpoint trails follow the fitted figure. The constraint covers horizontal framing; it does not guarantee vertical containment for future vocabulary. Fitting can compress apparent travel near an edge.

Preview: `cargo run --release -j 4 -- --bpm 145 --seed 42`; catalog walkthrough: `cargo run --release -j 4 -- --move-demo --bpm 145`.

## V4: Personality and presentation

Add a small set of deterministic dance personalities that modify timing, amplitude, lean, and step length at phrase or move boundaries while sharing the same choreography. A 60-second reel has already been exported; this remaining work extends its presentation and does not block export.

**Acceptance:** personalities are visibly distinct, repeatable from their settings, and preserve readable timing and grounded poses throughout the reel.
