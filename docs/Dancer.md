# Procedural dancer

## Purpose

Document the staged design and acceptance criteria for the procedural stickman dance sketch.

## Design

The choreography expresses intent as target joint angles; the body executes those targets, with physics added only after the pose-driven dance works. Keep the choreography representation independent of its execution backend so the same targets can be rendered kinematically in V1 and sent to physics motors in V2.

The presentation is a minimal stickman in side view, alone on a floor line. A beat clock drives pose timing and eased transitions through an eight-beat phrase. The initial move vocabulary is deliberately small; later phases expand it. Variation modifies choreography parameters at phrase or move boundaries rather than injecting random forces each frame.

The intended data flow is:

```text
beat clock → choreography timeline → desired joint angles
           → kinematic rendering (V1) or physics motor targets (V2+)
           → rendered stickman
```

Avian is the planned Bevy-integrated physics engine for V2. Check its current Bevy compatibility and exact crate release before adding the dependency.

## V1: Kinematic pose loop

Build the skeleton from rigid limb segments and joints. Represent poses as local joint-angle targets and interpolate between them with easing on a beat clock. Make one seamless eight-beat phrase from a few readable moves, such as a bounce, alternating step, arm swing, and squat. Use kinematic foot planting and floor constraints; do not add physics or randomized variation.

**Acceptance:** the figure reads as dancing, pose transitions do not snap, planted feet do not slide noticeably, feet stay above the floor, and the phrase returns smoothly to its starting pose.

## V2: Physics execution

Add Avian 2D bodies, colliders, joints, a floor, and motors that track V1's desired joint angles. Use fixed-step physics, bounded motor effort, damping, joint limits, and a simple balance response informed by support-foot contact. Keep the V1 choreography unchanged while tuning the physical execution.

**Acceptance:** the dancer completes the V1 phrase without collapsing, joint instability, or losing the beat; physics adds visible momentum without making the pose targets unreadable.

## V3: Choreography vocabulary

Expand the pose and move library to include moves such as a torso lean, kick, jump, and spin. Arrange moves into longer beat-based phrases with transitions, anticipation, and recovery; give jumps and spins specific entry and landing/facing transitions instead of treating them as ordinary pose blends.

**Acceptance:** the longer routine remains readable, transitions feel intentional, and the dancer recovers into stable support after high-energy moves.

## V4: Personality and presentation

Add a small set of deterministic dance personalities that modify timing, amplitude, lean, and step length at phrase or move boundaries while sharing the same choreography. Once those variations remain physically stable, polish the figure and staging and produce the finished reel.

**Acceptance:** personalities are visibly distinct, repeatable from their settings, and preserve readable timing and balance throughout the reel.
