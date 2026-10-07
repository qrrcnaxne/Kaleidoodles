# Software Quality

## Purpose

Describe code-quality audit methods and tools, and keep an append-only record of audits that actually run; rerun triggers are listed in [Audit History](Audit%20History.md).

## Formal audits / tooling

Manual review only goes so far — these runnable tools mechanically check
parts of the codebase. Routine Rust checks and available optional tools are
listed in `AGENTS.md`; none of the tools below are wired into CI.

**High value — selected for use when relevant:**
- **`cargo audit`** — checks `Cargo.lock` against the RustSec advisory
  database. Advisories can change without project changes; its rerun
  trigger is listed in [Audit History](Audit%20History.md).
- **`cargo machete`** — finds dependencies declared but never used.
- **`cargo llvm-cov`** — actual code coverage, turning testability
  claims from asserted into measured.

**Medium value:**
- **`cargo outdated`** — stale dependency versions, distinct from
  `cargo audit`'s vulnerability focus.
- **`cargo geiger`** — counts `unsafe` usage across the dependency
  tree.

**Low value / not worth it for this project right now**: `cargo bloat`
(binary size doesn't matter, not distributed), `cargo license` (only
matters if distributed), `cargo miri` (only pays off if there's real
`unsafe` code directly in this codebase — check that premise first).

## Shortcuts audit

Run a manual sweep plus `clippy::unwrap_used`/`expect_used` scoped to
production code. Grep for `TODO`/`FIXME`/`HACK`/`XXX`/
`unimplemented!`/`todo!`. Record findings in the audit log below; its
rerun trigger is listed in [Audit History](Audit%20History.md).

## Audit log

**Every time a real audit actually runs, record it here — what ran,
what it found, what happened as a result.** Append, don't overwrite.

- **Docs-purpose audit:** Reviewed the docs against their purposes; found that `Ledger.md` included project and recording infrastructure, beyond its intended art-piece scope. Added purpose sections, narrowed the ledger to the flow-field sketch, and separated rerun triggers from audit methods and run results.
- **Documentation synchronization and language audit (2026-10-07):** Reviewed all repository Markdown against file purposes, current project behavior, and the writing guidelines; synced the README's implemented-idea checklist and added the Shan Shui Inf reference, clarified recording reproducibility and the demo command in `AGENTS.md`, updated the available-tool notes, created the purpose-scoped dancer design note linked from the four queue phases, and confirmed the queue contains no fish-tank work.
