# Audit History

## Purpose

Index recurring audit kinds and the conditions that trigger another run. `Rerun when` describes a condition, not a calendar schedule; keep one entry per audit kind and update it rather than recording each occurrence. Actual findings and outcomes belong in the [Software Quality audit log](Software%20Quality.md#audit-log).

| Audit | Rerun when |
|---|---|
| Documentation/code drift | Code, behavior, or project structure changes in a way that may make docs stale; at project milestones; or whenever a docs audit is requested. A named area does not limit the full pass. |
| Abandoned-fix residue | A fix or experiment is abandoned, reverted, or superseded; check touched code and generated artifacts for remnants of the failed approach. |
| Whole-project Rust quality | At project-wide maintenance passes, before a release, or after Rust toolchain, feature, or build-configuration changes; run the full checks described in `AGENTS.md`. |
| Dependency security | After dependency changes and periodically even when dependencies are unchanged, because new RustSec advisories can affect the existing lockfile. |
| Build-artifact size | At maintenance passes, after unusually large builds, or when disk usage becomes a concern; inspect `target/` for unexpected growth. |
| Shortcut review | A design shortcut is proposed, adopted, or found during implementation; use the procedure in `Software Quality.md`. |
