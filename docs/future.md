# Future directions

## Purpose

Record incomplete capabilities and exploratory directions that are deferred from the current reel workflow.

## Incomplete: live microphone tempo

Live microphone beat tracking is experimental and incomplete. In the live test recorded on 2026-10-07, it took about ten seconds to lock and reported 96.8 BPM for music expected to be around 140–150 BPM. The detector and adaptive volume gate need further real-audio testing and tuning; use the explicit `--bpm` input for reel production in the meantime.

## Brownian motion

Explore a Brownian-motion sketch based on random particle motion. Its visual treatment and implementation scope remain undecided.

## Real-physics experiment

Experiment with real physics as an alternative artistic treatment for the dancer, comparing a small physics-driven phrase with the existing kinematic choreography. Keep the kinematic version as the visual baseline and add a physics integration only if the physical motion improves the artwork; implementation requires a separate design decision. Verify the current Bevy-compatible physics-engine release before selecting or adding a dependency.

## Autonomous escape-room game

The autonomous escape-room game idea remains deferred; revisit it only after a separate scope and architecture discussion.

## Procedural traditional art

Explore generating recognizable forms of traditional human art through authored rules, materials, and techniques. This direction is broader than psychedelic patterns. Deterministic systems can generate varied artworks through parameters and seeds. Study how the techniques work rather than copying their visual outcomes, and preserve distinctions between cultural styles. Possible studies include ink landscapes, botanical Gongbi illustration, calligraphy and type, woodcut or engraving, human architecture, and ornamental design.

References for technique-oriented exploration: [Shan Shui Inf](https://github.com/LingDong-/shan-shui-inf) (Chinese landscapes), [Non-Flowers](https://github.com/LingDong-/nonflowers) (Gongbi-inspired forms using mathematical functions and noise, without image assets), [Fishdraw](https://github.com/LingDong-/fishdraw) (line drawings), [Computer Grass](https://github.com/LingDong-/computer-grass) (algorithmic calligraphy), [Duct Tape Font](https://github.com/LingDong-/duct-tape-font) (force-directed brushstrokes), [Bad Cut Font](https://github.com/LingDong-/bad-cut-font) (simulated wood carving), and [Dither Sketches](https://github.com/LingDong-/dither-sketches) (facades, cityscapes, and palace architecture).
