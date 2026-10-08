# Agent instructions

## Rust validation

Run these from the repository root after Rust changes and report results:

```sh
cargo fmt --all -- --check
cargo check -j 4 --all-targets --all-features
cargo clippy -j 4 --all-targets --all-features -- -D warnings
```

Run `cargo test -j 4 --all-targets --all-features` when relevant; tests are runtime validation, not static analysis. Check important mutually exclusive feature combinations separately. For public API or documentation changes, also run `RUSTDOCFLAGS="-D warnings" cargo doc -j 4 --no-deps --all-features`.

Use `-j 4` for Cargo commands that compile/build. `cargo fmt` and utility subcommands such as `cargo sort` do not accept a job limit and do not compile the app. Report checks that could not run rather than silently skipping them.

Never execute binaries from `target/` directly; always run the app through Cargo (`cargo run --release -j 4 -- ...`). One exception: scripts that fan out many short-lived runs in parallel (such as `scripts/sweep.sh`) may invoke the Cargo-built release binary themselves, because concurrent `cargo run` calls serialize on Cargo's build lock and lose the parallelism.

## Bevy-aware lint guidance

The project uses Bevy 0.19.1. The latest `bevy_lint` release checked (0.6.0) supports Bevy 0.18 and requires its pinned nightly toolchain, so it is not compatible with this project. Check for a newer compatible release before considering it; do not install it without approval.

Bevy systems can trigger Clippy's `too_many_arguments` and `type_complexity` lints. Prefer extracting a `SystemParam` or introducing query/type aliases where that improves readability. If the shape is justified, use a narrowly scoped `#[allow(clippy::...)]` with a reason; don't globally suppress either lint. Keep Clippy's default lint set and `-D warnings`; don't enable `pedantic`, `nursery`, or `restriction` wholesale without a project-specific reason. Review any `--fix` diff.

Possible Cargo lint policy (proposal only; not yet added to `Cargo.toml`):

```toml
[lints.rust]
unsafe_code = "forbid"
```

This applies to this project's crates, not third-party dependencies; reconsider if authored unsafe code is genuinely needed.

## Optional analysis tools

Use relevant tools when installed; don't install tools or add project configuration without a reason and approval.

- `cargo machete` is the first choice for likely unused dependencies; `cargo-shear` is an alternative. `cargo udeps` is a deeper alternative requiring nightly.
- `cargo llvm-cov` measures test coverage; use it when coverage data would inform test design rather than as a routine build check.
- `cargo audit` checks locked dependencies against RustSec advisories. Configured `cargo deny check` can additionally enforce advisory, license, duplicate-version, and source policies.
- `typos` checks spelling; `cargo sort --check` checks Cargo.toml table ordering.
- `cargo hack check --feature-powerset` can exercise feature combinations once the project has meaningful features; select combinations carefully to avoid an excessive build matrix.
- `cargo geiger` inventories unsafe usage in the project/dependency graph; treat it as an audit aid, not proof of unsoundness or safety.
- Miri is dynamic UB detection and Kani is focused model checking, not general-purpose static linting. Use only when relevant and toolchain-compatible. `cargo-semver-checks` is for published library API compatibility, not this binary app unless it gains a public library API.

## Recurring audits

Run these proactively at sensible project milestones, without waiting for a prompt:

- Check for documentation/code drift and sweep for leftovers from abandoned or superseded fixes.
- Review `target/` size and run the whole-project formatting, check, Clippy, documentation, and Cargo manifest-order checks; use the commands and flags in the Rust validation section plus `cargo sort --check`.
- Before asking for commit approval, sync docs that describe changed behavior or structure and correct stale references.
- A `docs audit` request with a named area means a full pass over `docs/`; the named area is an emphasis, not a scope limit.

See [the audit index](docs/Audit%20History.md) for rerun triggers and [Software Quality](docs/Software%20Quality.md) for audit methods and the append-only run log.

## Tool availability checked in this environment

Available: stable Rust with rustfmt and Clippy; `cargo-sort` 2.0.1, `cargo-machete` 0.9.2, `cargo-audit` 0.22.2, and `cargo-llvm-cov` 0.9.0. Not found: `typos`, `cargo-shear`, `cargo-hack`, `cargo-geiger`, `cargo-deny`, `cargo-udeps`, `cargo-semver-checks`, Kani, and Bevy CLI/linter. Miri's Cargo shim exists, but its component is not installed for the active stable toolchain. Recheck availability in a new session.

## Model recommendations and experiments

When the user asks which model to use for a task, recommend one based on the task and suggest an alternative to try when a comparison would be useful. Help design a fair, small experiment using the same task/spec, repository state, tools, and effort; evaluate outcomes against task-specific criteria and project checks, noting time, cost, and failure modes. Treat results as evidence for this user's workflow, not a universal ranking, and use them to refine future recommendations. Do not switch models or run comparative calls without the user's direction.

## Project and implementation conventions

- This is a creative-coding app using Rust and Bevy. For non-trivial work, establish the intended visual behavior or user capability first; prefer the simplest approach that serves it over speculative infrastructure.
- Keep crate/module responsibilities and dependency direction clear. Use `name.rs` beside a `name/` submodule directory, not `name/mod.rs`.
- Update existing docs when a change makes them inaccurate, but don't duplicate detailed documentation in this file.
- Tests should exercise production-relevant behavior; don't add test-only bypasses. The user performs interactive visual GUI testing. Don't automate GUI input or screenshots unless explicitly asked.
- An earlier setup request excluded nannou for that task. Do not treat that as a permanent architectural prohibition: confirm the current preference and verify version compatibility if choosing or adding it becomes relevant.

## Creative-coding starting point and architecture reviews

- The initial 2D flow-field experiment in `src/sketches/flow_field.rs` established the project's first CPU-based vertical slice. For later sketches, start with the smallest sketch-specific CPU slice and add only the camera and systems it needs; defer generalized infrastructure until an experiment motivates it.
- Proactively prompt the user for an architecture review at structural inflection points, not for every feature. Review before introducing a workspace/subcrate, generalized sketch registry or switching, broad rendering/parameter/UI infrastructure, or extracting shared code.
- Also prompt when a second sketch begins duplicating or needing existing camera, input, math, or rendering code, when shared state/lifecycle becomes awkward across experiments, or when a new direction (such as GPU rendering or audio) changes architectural assumptions. Briefly explain the pressure, options, and smallest useful adjustment; wait for explicit implementation instruction.

## Exporting video clips

Always obtain explicit user approval before starting a video export, including recording frames for that export; discussing a reel or specifying its duration, frame rate, or appearance is not export approval. Prepare the implementation and preview first, then ask for approval of the concrete export settings.

Follow the [README export workflow](README.md#export-a-reel); use `cargo run --release -j 4 -- ...` for captures to avoid debug-build PNG encoding overhead.

- Keep exported `.mp4` files under `data/<sketch-name>/`, name them for what they show, and never commit generated artifacts.
- Use a fresh frame directory for each capture, match ffmpeg's input rate to `--fps`, and check the encoded video before removing its source frames.
- Recording is headless but requires a GPU, uses a fixed simulation timestep, and renders at 1080 × 1920; output fps does not guarantee real-time capture speed.
- Pass explicit `--bpm` for dancer reels; headless runs never open the microphone and remain neutral without manual tempo. Add `--seed` to reproduce choreography.
- Fish-only demo, simulation, and sweep commands require selecting `FishTank` in `src/sketches.rs`; see the [simulation limitations](README.md#fish-simulation-and-sweeps).

## Queue convention

Use one canonical queue only: `docs/Queue.md`. Keep each item to exactly one sentence; put implementation/design detail in the relevant project documentation and link to it from the queue. Delete completed items rather than marking them done. Don't split the queue by topic or create a second queue.

## Docs organization

Each file in `docs/` begins, immediately after its title, with a `## Purpose` section that states its one specific purpose. Keep content in the file whose purpose it serves; don't duplicate detail across files. `docs/Ledger.md` catalogs only the generative art pieces built in the repo (the sketches themselves), not infrastructure, process rules, or plans. Before adding a doc, define its purpose; if content fits no existing file's purpose, reconsider the placement before creating a new file.

## Git and collaboration

- Work on `master` only. Don't create or switch to another branch.
- Get explicit confirmation before every commit or push.
- Discussing or settling a design is not authorization to implement it; wait for an explicit implementation instruction. A question is not a command.

## Writing style

Be concise and precise. Prefer current-state descriptions over routine historical narration; include history only when it explains a real incident or prevents a repeated mistake. Don't quote the user's wording verbatim when a direct paraphrase works.
