# Future directions

## Purpose

Record incomplete capabilities and exploratory directions that are deferred from the current reel workflow.

## Incomplete: live microphone tempo

Live microphone beat tracking is experimental and incomplete. In the live test recorded on 2026-10-07, it took about ten seconds to lock and reported 96.8 BPM for music expected to be around 140–150 BPM. The detector and adaptive volume gate need further real-audio testing and tuning; use the explicit `--bpm` input for reel production in the meantime.

## Real-physics experiment

Experiment with real physics as an alternative artistic treatment for the dancer, comparing a small physics-driven phrase with the existing kinematic choreography. Keep the kinematic version as the visual baseline and add a physics integration only if the physical motion improves the artwork; implementation requires a separate design decision. Verify the current Bevy-compatible physics-engine release before selecting or adding a dependency.
