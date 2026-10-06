# Software Quality

## Purpose

Describe code-quality audit methods and tools, and keep an append-only record of audits that actually run; rerun triggers are listed in [Audit History](Audit%20History.md).

## Formal audits / tooling

Manual review only goes so far — these are real, runnable tools that
mechanically check parts of the codebase instead of relying on memory.
`AGENTS.md`'s `fmt`/`clippy`/`doc` hygiene pass is the only one run on
a routine basis — none of these are wired into CI yet.

**High value — cheap, plugs a gap, adopted as current scope:**
- **`cargo audit`** — checks `Cargo.lock` against the RustSec advisory
  database. Advisories can change without project changes; its rerun
  trigger is listed in [Audit History](Audit%20History.md).
- **`cargo machete`** — finds dependencies declared but never used.
- **`cargo tree`** — dependency-graph review; checks the real crate
  graph against the edges stated in `AGENTS.md`.
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
