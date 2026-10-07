# Software Quality

## Purpose

Describe code-quality audit methods and tools, and keep an append-only record of audits that actually run; rerun triggers are listed in [Audit History](Audit%20History.md).

## Routine validation

Follow the commands and job limits in [AGENTS.md](../AGENTS.md#rust-validation): formatting, all-target/all-feature check and Clippy, relevant runtime tests, and rustdoc with warnings denied for public API or documentation changes. Maintenance passes also check manifest ordering and build-artifact size. These checks are run locally; no CI workflow is configured.

Static checks do not establish visual quality or real-audio tempo accuracy. The user performs interactive visual testing; record which runtime checks actually ran and what remains unverified.

## Optional tools

Use tools when their results answer a concrete question; do not install tools or add configuration without approval.

| Question | Tool | Limit |
|---|---|---|
| Known locked-dependency vulnerabilities | `cargo audit` | Advisory data changes independently of the lockfile. |
| Likely unused dependencies | `cargo machete` | Findings need review before removal. |
| Test coverage gaps | `cargo llvm-cov` | Coverage measures execution, not test quality. |
| Available dependency updates | `cargo outdated` | Newer versions are not necessarily compatible or required. |
| Dependency unsafe-code inventory | `cargo geiger` | Counts do not prove soundness or unsoundness. |
| Dynamic undefined behavior | Miri | Toolchain compatibility and supported operations constrain use. |
| Binary size or license policy | `cargo bloat`, license tools, or configured `cargo deny` | Select only when distribution or another concrete requirement warrants the analysis. |

Availability is recorded in [AGENTS.md](../AGENTS.md#tool-availability-checked-in-this-environment). Bevy-specific lint compatibility must be checked before use.

## Shortcuts audit

Review production paths for unchecked assumptions and stale `TODO`, `FIXME`, `HACK`, `XXX`, `unimplemented!`, and `todo!` markers with `rg`. Inspect each finding in context. If using Clippy's optional `unwrap_used` or `expect_used` lints, scope them to production code and review the results; routine checks keep the default Clippy lint set.

Record concrete findings and corrective actions below. Rerun triggers belong in [Audit History](Audit%20History.md).

## Audit log

**Every time a real audit actually runs, record it here — what ran,
what it found, what happened as a result.** Append, don't overwrite.

- **Docs-purpose audit:** Reviewed the docs against their purposes; found that `Ledger.md` included project and recording infrastructure, beyond its intended art-piece scope. Added purpose sections, narrowed the ledger to the flow-field sketch, and separated rerun triggers from audit methods and run results.
- **Documentation synchronization and language audit (2026-10-07):** Reviewed all repository Markdown against file purposes, current project behavior, and the writing guidelines; synced the README's implemented-idea checklist and added the Shan Shui Inf reference, clarified recording reproducibility and the demo command in `AGENTS.md`, updated the available-tool notes, created the purpose-scoped dancer design note linked from the four queue phases, and confirmed the queue contains no fish-tank work.

- **Procedural dancer completion audit (2026-10-07):** Reviewed the interrupted seeded-generator implementation and dancer documentation; corrected mismatched exit/playback phases, discontinuities between generated moves, and loop travel/turn handling, and synced the design note, ledger, and queue. Formatting, all-target/all-feature check, Clippy with warnings denied, all 74 tests, rustdoc with warnings denied, manifest ordering, and whitespace checks passed. A three-second headless run at 30 fps with BPM 145 and seed 42 passed with GPU access after the sandbox could not detect a GPU. Visual acceptance remains pending; `target/` measured 24 GB during reconnaissance.

- **Dancer reel framing fix (2026-10-07):** Added viewport-dependent silhouette fitting with six-percent horizontal margins so accumulated moonwalk travel stays inside the portrait frame; the rig translates as a whole and only scales around floor height when its stance is too wide to fit. Tests sample five seeds over two loops in a 1080 × 1920 viewport and check normal-pose preservation and whole-rig translation. All 76 tests, formatting, all-target/all-feature check, Clippy with warnings denied, rustdoc with warnings denied, manifest ordering, and whitespace checks passed. Synced dancer documentation and removed the visual-review queue item after user acceptance; visual confirmation of the framing adjustment remains with the user.

- **Full documentation audit (2026-10-07):** Reviewed every document under `docs/`, plus README and AGENTS, against implementation, file purposes, queue rules, and current user decisions. Removed the abandoned Pi research and ideas documents at the user's request and removed their links. Consolidated capture commands in README with release builds, fixed-step and warm-up semantics, GPU requirements, silent-video output, and frame cleanup guidance; documented fish observation cadence, final-result restrictions, immigration limits, demo founder count, and sweep output replacement. Simplified dancer and ledger descriptions, removed the already-exported reel from remaining personality work, and corrected optional-tool guidance. All remaining docs have immediate Purpose sections; 27 local Markdown links and anchors passed validation. Formatting, all-target/all-feature check, Clippy with warnings denied, rustdoc with warnings denied, manifest ordering, and whitespace checks passed. Runtime tests and GUI checks were not rerun for these documentation-only changes. `target/` measured 24 GB; no build artifacts were deleted. Earlier audit entries are preserved as historical results, including pending visual checks recorded at the time.
