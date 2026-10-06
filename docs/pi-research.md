# Pi setup and capability research

## Purpose

Document what is installed in the Pi setup, why, what was rejected, and what remains open.

Written 2026-10-03. Pi 1.0.0, default provider `opencode`, default model `gpt-6-luna` (1.05M-token window). Pi's core is small (read, bash, edit, write); everything else is an added capability. Facts marked **verified** were checked on disk or by running the shipped code. Anything marked **untested** has not been exercised in a live Pi session.

## Current state

### Installed packages

All pinned to exact versions in `~/.pi/agent/settings.json` (`pi-web-access` was unpinned at first and was pinned to 0.35.0 on 2026-10-03).

| Package | Version | Role | State |
|---|---|---|---|
| `pi-web-access` | 0.35.0 | Web search and page fetch | Active. Tools load through `web_enable` in new sessions. |
| `@gotgenes/pi-permission-system` | 39.0.1 | Allow/ask/deny rules for tools, bash, paths, MCP | Active. |
| `@juicesharp/rpiv-ask-user-question` | 2.12.0 | `ask_user_question` terminal dialogs | Active. Dialog untested. |
| `pi-powerline-footer` | 0.19.1 | Readable status footer | Active. Rendering untested. |
| `pi-observational-memory` | 3.1.4 | Continuous memory, compaction summaries | Active. |
| `billion-context-pi` | 0.1.83 | Model-driven context compression | Installed, **disabled** via `~/.pi/acp.json`. |
| Pi example `plan-mode` | copied from Pi 1.0.0, then customized | `/plan` read-only mode | Active. Changes 1 to 7 applied (see capability 8). Offline tests passed; live flow untested. |

### Config files

| File | Purpose |
|---|---|
| `~/.pi/agent/settings.json` | Package list, `powerline` block, `enableInstallTelemetry: false`, `enableAnalytics: false`. |
| `~/.pi/agent/extensions/pi-permission-system/config.json` | Permission rules (mode 600). Review log lands in a `logs/` directory beside it. |
| `~/.pi/acp.json` | `enabled: false`, `autoUpdate: false`, `delegate: false` for billion-context-pi. Lives in `~/.pi`, not `~/.pi/agent`. |
| `~/.pi/agent/extensions/powerline-footer/theme.json` | Footer color overrides, aligned with the Tokyo Night theme. |
| `~/.pi/agent/themes/tokyo-night.json` | Active Pi theme (`"theme": "tokyo-night"` in `settings.json`): Tokyo Night colors adapted for a black terminal; see "Theme" under capability 4. |
| `~/.config/kitty/kitty.conf` and `current-theme.conf` | Tokyo Night colors for kitty with **no `background` line**, so kitty keeps its default black. Layout is the one `kitten themes` uses. |
| `~/.pi/agent/extensions/plan-mode/` | `index.ts` and `utils.ts` (customized), plus `index.ts.orig` and `utils.ts.orig`, which are byte-identical to Pi's shipped example in `/usr/lib/node_modules/pi/packages/coding-agent/examples/extensions/plan-mode/`. Use `diff` against the `.orig` files to see every change. |
| `~/.pi/agent/keybindings.json` | `tui.editor.deleteWordBackward` is `ctrl+w`, `alt+backspace`, and `ctrl+backspace` (added 2026-10-06, to match Claude Code). `/reload` applies edits. In kitty, `ctrl+backspace` arrives as a distinct sequence, **verified** with Pi's own key matcher and keybinding manager. The same key also deletes a session in the session picker (`app.session.deleteNoninvasive`), a separate screen. Live behavior untested. |
| `~/.pi/agent/mcp.json` | Does not exist. No MCP servers configured. |

Logs: observational memory writes `observational-memory/debug.ndjson` under the agent directory; billion-context-pi writes `~/.pi/acp.log`.

## Capability 1: guardrails on bash and file access

**Decision:** `@gotgenes/pi-permission-system@39.0.1`, no sandbox.

**Why.** Pi runs with the user's full permissions and does not ask before tool calls (Pi's security docs). A cheap model with a bash tool and web access needs a gate. The docs rank isolation above monitoring; the user declined a sandbox, so the gate is a guardrail, not a boundary.

**Sandbox options considered.** Docker (installed, see below), `pi-sandbox` (bubblewrap; `bwrap` and `rg` are present), Pi's containerization guide. Declined by the user. Docker stays installed and enabled. `pi-sandbox` would be redundant under a container.

**Audit of the package.** Readable TypeScript, no lifecycle scripts, no network code, no telemetry, no environment reads, npm provenance attested. One subprocess: `npm root -g` with fixed arguments. Bash parsing uses `tree-sitter-bash` through WASM. npm blocked `tree-sitter-bash`'s `node-gyp-build` install script; harmless, because only the bundled WASM is used.

### Rule design (**verified** against the live config)

| Area | Setting |
|---|---|
| Default `*` | `ask` |
| `external_directory` | `ask` (any path outside the project) |
| Reading downloaded crate sources | `allow` through `external_directory_read` for `~/.cargo/registry/src` and `~/.cargo/registry/src/*`, and `~/.cargo/git/checkouts` and `~/.cargo/git/checkouts/*` (2026-10-06). Everything else under `~/.cargo` still asks: `config.toml`, `credentials*`, `env`, `bin`, `registry/cache`, `registry/index`, `advisory-db`. Writes into the sources still ask. The key sits after `external_directory` because that bare key expands first and later rules win. A trailing `*` is greedy; a bare directory pattern matches the directory entry only. `*.env` files inside crates (for example `skiplist-0.5.1/.env`) still ask. |
| Bash | 88 rules: 48 allow, 40 ask, 0 deny. Allows cover everyday read-only commands, file basics, `curl`/`wget`, cargo check/clippy/fmt/test/build/doc/tree/metadata/machete/audit/sort/outdated, and `ffprobe`/`ffmpeg` (added 2026-10-06). The other gates still apply to media commands: paths outside the project, protected paths (`.git`, `.pi`), credential paths, and output redirects ask; chains containing `rm` or `git push` ask; `ffplay` and `xargs`/`bash -c` wrappers still ask. |
| Bash asks | git commit/push/pull/checkout/switch/merge/rebase/stash/restore/reset/clean, `rm`, `mv`, `sudo`, `docker`, `cargo run/install/add/remove/update`, `npm`, `npx`, `pip`, `pi`, `env`, `printenv`, `find -exec/-delete`, `bash -c`, `sh -c`, `curl` uploads (`-d`, `--data`, `-F`, `--upload-file`). |
| Bash denies | None. The three `rm -rf` denies on `/`, `~`, and `$HOME` became `ask` on 2026-10-06: the patterns `rm -rf /*` and `rm -rf ~*` matched every `rm -rf` with an absolute path or under the home directory, not just the catastrophic cases, so they blocked ordinary use such as `rm -rf /tmp/build`. Every `rm`, including `rm -rf`, now asks; nothing is allowed unconditionally. The three entries are kept as explicit `ask` rules in case `rm *` is loosened later. **Verified** with the offline harness: 22 `rm` forms (root, home, absolute and relative paths, chains, substitutions, `sudo`, `xargs`, `bash -c`, `find -delete/-exec`) all ask. |
| Credential paths | `ask`: `.env*` (except `.env.example`), `~/.ssh`, `~/.aws`, `~/.gnupg`, `~/.config/gh`, `~/.config/gcloud`, `~/.netrc`, `~/.git-credentials`, `~/.npmrc`, `~/.cargo/credentials*`, `~/.docker/config.json`, Pi's `auth.json` and `web-search.json`, browser profiles. |
| Protected writes | `ask` for `.git/**`, `.pi/**`, `.cargo/*`, and `~/.pi/agent/extensions/**`, set on both the `write`/`edit` surfaces and `path_write`. |
| MCP | `mcp: {"*": "ask"}` |
| Explicit tool allows | `web_search`, `fetch_content`, `get_search_content`, `source_check`, `web_enable`, `ask_user_question`, `recall`, `compress`, `decompress`, `search_context`, `acp_status`, `acp_cache` |
| `yoloMode` | `false` |

The user is in the `docker` group, which is root-equivalent, so `docker` is `ask`: an agent with docker access can bypass every other rule.

### Findings worth keeping

- **Chains are split.** The parser splits `;`, `&&`, `||`, `|`, newlines, `$(...)`, and subshells; each part is checked and the most restrictive result wins. Wrappers (`bash -c`, `eval`, `sudo`, `env`, `xargs`, `find -exec`) and unparseable commands fall to `ask`. Quoting tricks (`g''it push`, `git\ push`) and unresolved variables also `ask`. The `*| sh*` patterns in the config are redundant: a bare `sh` unit already hits the default.
- **Bash writes are checked against `path_write`, not the per-tool `write`/`edit` surfaces.** Rules placed only under `write`/`edit` left `echo x > .git/hooks/pre-commit` allowed. Protections must be on `path_write` to cover bash.
- **New tool names hit the default `ask`.** An extension's tools need explicit allow entries. `web_enable` was missed at first and would have prompted each session.
- **Session approvals are memory-only** (never written to disk). `save()` rewrites only three runtime knobs (`debugLog`, `permissionReviewLog`, `yoloMode`) and is user-invoked.
- **Project `.pi` config loads only for trusted projects**, and project config overrides global config, so a repo you did not write could ship rules that loosen yours.
- **Real-session check.** The user confirmed the permission rules behave. The first probes (`ls; echo PROBE`, `cargo check && echo PROBE`, `echo $(ls)`) were all allowlisted, so silence was the correct result.

### Testing rules offline (method)

The test scripts are not kept in the repo. They lived in the session scratchpad under `/tmp` and were lost at a reboot, then rebuilt on 2026-10-06 (about 100 lines: three gates, a table of expected verdicts, a scratch `agentDir`). Keeping a copy in the repo is an open idea in `docs/pi-ideas.md`.

Load the extension's policy engine through Pi's own `jiti` (`/usr/lib/node_modules/pi/node_modules/jiti`) with `@earendil-works` aliased to Pi's bundled packages and `PermissionManager` pointed at a scratch `agentDir` holding a candidate config. Replicate the three real gates: command (`resolveBashCommandCheck` after `BashProgram.parse`), path (`path_read`/`path_write` through `PathNormalizer`), and external directory. Two traps: a tool-intent probe for `write`/`edit` tests a code path real calls never use (the real gate emits path intents), and `moduleCache: false` splits module state so the parser never warms and chains look unsplit. Validate every candidate against the package's shipped `schemas/permissions.schema.json` before deploying.

### Weak points

- It is a guardrail, not a boundary. A command the rules do not match can still run once approved.
- Prompt fatigue: anything outside the project directory prompts, including `/tmp`. If it becomes tiresome, an `external_directory` rule for `/tmp/**` may work; the schema support was not checked.
- Never use session-wide approval on prompts for credentials, `.pi/**`, or the extension config directory.
- Optional hardening not applied: make the config directory root-owned so Pi cannot rewrite the rules, then edit with `sudoedit`. The extension may write its own review log there, so a log path change or a lock on `config.json` alone (`chattr +i`, filesystem permitting) would be needed.

### Docker note (history that prevents a repeat)

After a kernel package update without a reboot, `/usr/lib/modules/<running kernel>` disappears and the Docker daemon cannot load `overlay`, so it fails with a start-limit error. A reboot fixed it. Docker 29.8.2 runs and `hello-world` passes.

## Capability 2: live code diagnostics (LSP)

**Decision:** skipped.

**Why.** Per-edit diagnostics add tokens on every edit, and `cargo check` and `clippy` (required by `AGENTS.md`) already report the same compile errors; an agent that fixes all errors in one pass gains little from incremental feedback. Rust-specific costs: `rust-analyzer` is not installed (the `~/.cargo/bin` entry is only rustup's proxy), it compiles the whole Bevy project before reporting, and it runs `cargo check` against the same `target/` directory, contending for the build lock and ignoring the `-j 4` rule.

**Candidates reviewed.** `pi-lsp-extension` (auto-appends up to 10 error lines per file after write/edit, plus navigation tools), `@narumitw/pi-lsp` (on-request diagnostics, zero dependencies, server lifetime unclear), `lsp-pi` (older, repository link points at Pi's own monorepo), `pi-lens` (26 MB, auto-formats and auto-fixes files, installs dependencies; conflicts with the rule to review `--fix` diffs).

**Revisit when** the codebase has many modules and grep stops being reliable for references, rename, or definition lookups.

## Capability 3: structured questions

**Decision:** `@juicesharp/rpiv-ask-user-question@2.12.0`.

**Why.** Terminal-only dialogs (up to four questions per call, single/multi-select, free text, confirmation, a submit review tab), the most used option (about 300K downloads a month), no server or port.

**Audit.** Readable TypeScript, no lifecycle scripts, no network, no `eval`, no telemetry. The only subprocess is the optional Ctrl+G external editor (argument array, temp file removed afterward). Depends on `@juicesharp/rpiv-config` (reads and writes `~/.config/rpiv-ask-user-question/config.json`). `@juicesharp/rpiv-i18n` is an optional peer and is not installed, so the UI is English only. No npm provenance attestation.

**Notes.** `ask_user_question` is allowed in the permission config. The tool removes itself in non-interactive mode, and the dialogs need a terminal at least 100 columns wide.

**Rejected.** `pi-interview` (local HTTP server, browser tab, recovery files), `@mazli/pi-ask-user-question` (older than Pi 1.0), and several smaller variants. Packages sharing the tool name `ask_user_question` cannot coexist.

## Capability 4: cost and usage visibility

**Decision:** `pi-powerline-footer@0.19.1`; skip usage and tracing packages.

**Why.** Pi already shows context, usage, and cost in the footer and `/session`, and session files record per-message cost (total across all sessions so far: under $0.09). The native footer was hard to read (grey `dim`/`muted` colors), so the powerline footer replaces the look, not the data.

**Audit.** Readable TypeScript, no lifecycle scripts, no `eval`, no telemetry. It polls `git status` (fixed arguments, read-only git environment, 200 ms timeout), fetches currency rates only for non-USD display (none on this setup), and starts a shell only when bash mode is toggled. Opt-in features left off:

- `autoFollowUp` is off by default and would send messages and the agent's latest text to `api.typesafe.ai` (needs a `TYPESAFE_API_KEY`). Keep it off.
- Working vibes (AI-generated loading messages) do nothing unless a vibe theme is set.

Also present: a local prompt queue (`powerline-footer/inbox.jsonl`), an editor stash, and `/cd`. No npm provenance attestation.

**Configuration.** `powerline` block in `settings.json` (`preset: default`, `placement: above`, `welcome: true`, `autoFollowUp: false`) plus `theme.json` overrides aligned with Tokyo Night: purple model (`#bb9af7`), cyan path (`#7dcfff`), yellow cost (`#e0af68`), and the theme's own `muted` and `dim` colors for tokens, context, separator, and border, so the greys follow the theme. The `thinking*` footer colors come from the theme, not from overrides. See "Theme" below. Presets: `default`, `minimal`, `compact`, `full`, `nerd`, `ascii`; `/powerline <name>` switches, `POWERLINE_NERD_FONTS=0` forces ASCII.

**Rejected.** `@narumitw/pi-usage` (account balance and plan windows; its OpenCode support targets the Go subscription, and native cost data covers per-session spend), Langfuse/LangSmith/Braintrust/Raindrop tracing (heavy hosted services).

### Theme

**Status: Tokyo Night, colors only, no background (2026-10-03, third attempt).** The first attempt (Tokyo Night with its own navy background) and a second (Moonfly) were set up and then undone at the user's request. The current setup keeps the terminal's own black background: kitty gets Tokyo Night's text and ANSI colors with no `background` line, and the Pi theme was adapted for black. The notes below are history that prevents repeating mistakes.

- **Why a change.** On kitty's default black, Pi's built-in `dark` theme passes WCAG contrast but its secondary text is a low-saturation grey: body text 15.9:1, muted 8.4, `dim` 5.8. Pi themes color text and panels only; the terminal owns the background, so the best result pairs a Pi theme with the matching kitty theme.
- **Candidates measured** (contrast of muted / dim / faint border on each palette's own background, **verified** with Pi's own color code): Catppuccin Mocha 9.3 / 7.4 / 3.4, Gruvbox Dark 8.6 / 6.8 / 2.3, Tokyo Night 8.1 / 4.1 / 2.8, built-in `dark` 8.4 / 5.8 / 5.3 on black. Nord, Everforest, Dracula, and most of `@spences10/pi-themes` and `@xynogen/pix-themes` had weaker secondary text or borders. `pi-themes` ships an extension and lists no license.
- **Source.** The three bundle themes are data-only (the manifest declares `themes` and no extensions; no scripts). The chosen file was copied into `~/.pi/agent/themes/` after reading it, not installed as a package, and it passes Pi's `validateThemeJson`.
- **The one edit.** Tokyo Night's `dim` (`#737aa2`) measured 4.1:1 on its own background, below 4.5. It is now `#8991b6`: 5.5:1 on `#1a1b26`, still dimmer than muted (`#a9b1d6`, 8.1:1). Faint borders (`borderMuted`) remain low at 2.8:1; change that variable if borders look too faint.
- **kitty.** Palette file from the palette author's repository (MIT), `https://github.com/folke/tokyonight.nvim/raw/main/extras/kitty/tokyonight_night.conf`. Parsed with kitty's own loader (**verified**: background `#1a1b26`, foreground `#c0caf5`). A reload signal was sent to the running kitty processes; whether the colors applied is **untested visually**.
- **Tokyo Night on black (current).** Starting from the bundle's file: panels changed from lifted blue-grey (`#292e42` and similar) to subtle tints (user `#10131f`, pending `#0b0c12`, success `#0e1a14`, error `#1e1018`, custom `#17132a`, selected `#222a40`); `dim` `#8991b6`; comments `#8991b6`; thinking off `#7a83a8` and minimal `#9aa5ce` (they were both `#565f89`, 3.4:1 and identical); `export` page `#000000`. Low, medium, high, and xhigh keep Tokyo Night's blue, cyan, purple, and red. It passes `validateThemeJson`. On black and on the user, tool-success, and tool-error panels every text role is at least 4.8:1 (text 13.0, muted 9.9, dim 6.8, accent 8.3); dim text on a selected row is 4.6. Faint borders stay at 3.4:1 on black. To drop the panel tints entirely, set the six panel variables to `""` (terminal default).
- **Process slip, caught.** The first kitty install used the wrong archived file (Moonfly's colors); kitty's own loader exposed it (foreground `#bdbdbd` instead of `#c0caf5`), and it was corrected and re-verified: foreground `#c0caf5`, red `#f7768e`, blue `#7aa2f7`, background `0,0,0`. Always parse the installed kitty config with kitty's loader after writing it.
- **Why Moonfly (pure black).** The user wanted a pure-black background. kitty-themes has 26 palettes with `#000000` and two near-black; ranked by the legibility of their six hues, the best were Mishran, IR Black, Vesper, Modus Vivendi, Harper, Tomorrow Night Bright, and Moonfly (`#080808`, text `#bdbdbd`, weakest hue 6.7:1). Pro, Homebrew, and ANSI 1987 have blues at 1.6 to 2.1:1 and are unreadable on black. Moonfly was chosen by preference.
- **Finding: `system` does not fix the grey.** Pi's `system` theme gives every black palette the same neutral grey for muted (`#9d9d9d`) and dim (`#7f7f7f`) text, **verified** by running Pi's own `generateSystemThemeColors` on each palette. A black kitty theme alone therefore leaves the flat grey, so a custom Pi theme is needed.
- **The Moonfly Pi theme.** Built from Pi's `system` output for the palette, then overridden: the palette's exact accent colors (purple `#cf87e8`, blue `#80a0ff`, green `#8cc85f`, red `#ff5d5d`, yellow `#e3c78a`, cyan `#79dac8`); tinted neutrals (muted `#a3acc4`, dim `#8790ab`, faint border `#566078`); and subtle panel lifts from near-black (user message `#131722`, tool pending `#101010`, success `#0f1a12`, error `#1f1112`, selected `#232838`). `text` stays the terminal default. It passes `validateThemeJson`. Contrast on `#080808`: text 10.7:1, muted 8.8, dim 6.3, accent 7.9, faint border 3.2; every text role stays at or above 4.5 on the user-message, tool, and error panels. Dim text on a selected row is 4.6.
- **Thinking-level colors (a miss, fixed).** The first version of the Moonfly theme left these at Pi's generated defaults: off `#6a6a6a` (3.7:1, the flat grey again), minimal `#4f6ac4`, low `#5571cc`, medium `#3f8e81` (a dull teal). They color the footer's `think:` segment and the prompt border, so they showed up as dim teal text and a teal editor border in a screenshot of the real session. They are now readable and graded: off `#7d86a0`, minimal `#8a9fd0`, low `#80a0ff`, medium `#8cc85f`, high `#cf87e8`, xhigh `#ff87c0`, max `#ff5d5d` (5.5:1 to 10.1:1 on `#080808`). The powerline `thinking` override was removed so the footer follows the theme. When building a Pi theme, define every `thinking*` role, not only the neutrals.
- **Footer chevrons.** The `>` separators between footer segments are a hard-coded ANSI 256 grey (`sep: 244`, `#808080`, 5.1:1) in `pi-powerline-footer`'s `colors.ts`. The `separator` key in the footer's `theme.json` does not change them. Left as is: the package is audited and pinned, and patching it is not worth a subtle chevron.
- **Verification method.** A screenshot of the real session was decoded with a small standard-library PNG reader and pixel colors were compared with the installed files: terminal background, panels, model, tokens, and cost matched. A mock preview page is not a substitute: the earlier preview drew the plan-mode footer and a blue editor border, while the real session uses the powerline footer and a thinking-level border color.
- **kitty.** Palette file from the palette author's repository (MIT), `https://github.com/bluz71/vim-moonfly-colors/blob/master/extras/moonfly-kitty.conf`, taken from the kitty-themes collection. Parsed with kitty's own loader (**verified**: background `#080808`, foreground `#bdbdbd`). A reload signal went to the running kitty; the visual result is **untested**.
- **To redo a theme.** Pi: create `~/.pi/agent/themes/<name>.json` and set `theme`. kitty: `kitten themes`, then a reload signal. Define every role, including the `thinking*` ones, and check a screenshot of the real session, not a mock.
- **Preview page.** A private mock-session preview of the candidates was published as an artifact during the comparison. It is not part of the setup.

## Capability 5: MCP

**Decision:** use Pi's built-in MCP; install nothing.

**Why.** MCP is bundled (`builtin:mcp`, with `builtin:codemode` and `builtin:tool-search`). Commands: `pi mcp add|remove|list|login|logout`. Config: `~/.pi/agent/mcp.json`, and project `.pi/mcp.json` only for trusted projects. Transports: stdio and streamable HTTP, with OAuth (tokens in `~/.pi/agent/mcp-auth.json`). Exposure modes: `codemode` (default), `deferred`, `direct`, `hidden`; results over 20 KB are truncated for the model. The token-overhead problem that `pi-mcp-adapter` (1.4M downloads, 4.4 MB, 15 dependencies) solves is covered natively, and an extension that registers `/mcp` replaces the built-in one.

**Permissions.** MCP tools are named `mcp__<server>__<tool>` and gated on the `mcp` surface; the config has `mcp: {"*": "ask"}`. A per-server allow looks like `"mcp": {"*": "ask", "docs": "allow"}` (**untested**, no server configured).

**Caveats.** A stdio server runs arbitrary code as the user: vet it like a package and pin versions instead of `npx -y …@latest`. Tool results are an injection channel. Prefer the user-level file for anything with credentials, `deferred` or `codemode` exposure, and `toolExposure` to hide destructive tools.

**Rust docs servers** (`mcp-rust-docs`, a docs.rs MCP, `cratedocs-mcp`) exist. `fetch_content` can already read docs.rs, so try that first; revisit if Bevy API errors persist.

### Telemetry and network defaults

`enableInstallTelemetry` (default on: anonymous install/update reporting and provider attribution headers) and `enableAnalytics` (default off) are both set to `false`, **verified** with Pi's own `isInstallTelemetryEnabled`. The `pi.dev` latest-version request is separate and is controlled only by `PI_SKIP_VERSION_CHECK=1`, which is **not set, by decision** (the version check stays on; shell rc untouched). With it set to any non-empty value, Pi skips a `GET https://pi.dev/api/latest-version` request made at interactive startup, which sends only the user agent `pi/<version> (<platform>; node/<version>; <arch>)`; the effect is that no "new version available" notice appears and updates are checked by hand. It does not control the telemetry settings above. `PI_OFFLINE` disables all automatic network activity, including model catalog refresh, so it is not suitable. `PI_TELEMETRY` overrides the install-telemetry setting. `/share` and `/bug` upload only on command.

## Capability 6: context management

**Decision:** observational memory active; billion-context-pi installed but disabled; compare against long Claude Code sessions.

**Why.** The window is 1.05M tokens, so native compaction (`compaction.reserveTokens` 16384, `keepRecentTokens` 20000, per-model overrides under `compaction.modelOverrides`) rarely triggers; the largest session so far reached about 120K tokens with zero compactions. The real concern is summary-of-summary decay from repeated compaction in long sessions.

### The two installed tools

| | `pi-observational-memory@3.1.4` | `billion-context-pi@0.1.83` |
|---|---|---|
| Approach | Maintains observations and reflections continuously; at compaction renders prepared memory instead of re-summarizing | The model calls `compress` on message ranges; tiered summaries (T1 to T3); `decompress` and `search_context` retrieve |
| Relation to native compaction | Hooks `session_before_compact` and calls `ctx.compact()` itself; falls back to Pi's summarizer when it has nothing prepared | Registers `session_before_compact` returning `cancel: true`; becomes the sole context manager |
| Trigger | 81,000 raw tokens since the last compaction (default `calibrated` mode; a `ratio` mode of 0.68 of the window is available) | Nudges, as percentages of `modelContextLimit` (below) |
| Footprint | 218 KB, zero dependencies, provenance attested | 3.8 MB bundled |
| Tools and commands | `recall`, `/om:status`, `/om:view` | `compress`, `decompress`, `search_context`, `acp_status`, `acp_cache` |
| Model cost | Background observer/reflector workers use the session model (or a `model` setting); capped at 16 turns | Extra tools, system-prompt text, reference tags on every message |

**Audits.** Observational memory: no network code of its own, no `eval`, no telemetry, one subprocess (clipboard helper). Billion-context-pi: no `eval`, no telemetry, only the npm registry as a host. Findings: **auto-update was on by default** and runs `npm install` of newer versions (disabled with `autoUpdate: false`; an exact pin also skips it); it can spawn child Pi processes through delegate tools (disabled with `delegate: false`); it uses `execFile` for `npm`/`node` and `tmux` for terminal detection; it logs to `~/.pi/acp.log`.

### Billion-context-pi thresholds (read from the 0.1.83 source)

The author's `CONFIGURATION.md` link returns 404, so this came from reading the unminified bundle; treat it as unconfirmed behavior.

| Level | Default | Tokens on a 1.05M window |
|---|---|---|
| First nudge (needs enough compressible content) | 45% | about 472K |
| Over-limit nudges | 75% | about 787K |
| Emergency nudge and hard truncation of oldest content | 95% | about 997K |
| Growth nudges after the first | every ~50K tokens of compressible mass | |

Nudges are advisory (`force: soft`); the model must call `compress`. The usable limit is slightly below the window because output headroom is reserved. Controls, highest precedence first: env `ACP_MODEL_CONTEXT_LIMIT` (absolute tokens), `modelContextLimit` in `acp.json` (absolute tokens, a virtual window that all percentages apply to), and `compress` keys: `maxContextLimit` (takes a **percentage**, e.g. `"75%"`), `emergencyThresholdPercent`, `nudgeGrowthTokens` (absolute), `minPressureBenefitTokens`, with per-model overrides under `compress.providers.<provider>.models.<id>`. The first-nudge 45% is not independently configurable, and `minContextLimitPct` must not exceed `maxContextLimitPct`; to compress earlier on a large window, lower `modelContextLimit`. A small virtual window also lowers the hard-truncation point. Thresholds are currently left at defaults.

### The two cannot run together

Billion-context-pi's `session_before_compact` cancel can cancel the compactions observational memory triggers, and the author warns that two compressors corrupt each other's output. With `enabled: false` in `acp.json`, ACP returns at registration with no handlers and no tools (**verified** by loading the shipped code).

To switch to ACP: set `"enabled": true`, restart, and take observational memory out of the picture. Its `passive` setting stops its compaction trigger; whether it also stops its background workers was not confirmed, so removing the package is the safe route.

### Comparison plan (not yet run)

Give the same long task to each, and after several compaction cycles ask about a specific early decision. Compare how much exact detail (file names, reasons) survives. Observational memory runs on `gpt-6-luna`, so its quality depends on that model.

### Rejected

`context-mode` (Elastic License 2.0, targets tool-output bloat rather than summary decay, Claude Code-style hooks), `@astrosheep/pi-context` (note-and-reset workflow; writes under `~/.agents/notes`, outside Pi's directory, which would prompt under `external_directory`; its `dream` helper names an Anthropic model in its example config), `pi-hermes-memory` (cross-session memory, not within-session decay), `billion-context` (same objections as above).

### Where context actually grows

Fetched web pages. `pi-web-access` inlines up to 30,000 characters per fetch (`maxInlineContentChars`, capped at 200,000) and keeps the full text retrievable through `get_search_content`. **Decision: leave it uncapped,** meaning the package default applies (30,000 characters inline per fetch, full text retrievable). No `web-search.json` is created.

## Capability 7: subagents and model tiering (parked)

**Status:** parked by the user for later research, including whether a custom workflow is possible. Nothing here is a decision, and none of it has been used.

**Findings so far**

- Pi ships an official subagent example at `examples/extensions/subagent/` (one 36 KB `index.ts`, `agents.ts`, sample agents, workflow prompts). Each subagent is a separate `pi` process. Agent files are markdown with `tools:` and `model:` frontmatter; modes are single, parallel (max 8, 4 at once), and chain with a `{previous}` placeholder; it reports per-agent turns, tokens, cost, and model. It loads only user-level agents unless project agents are requested. Its sample agents name older models (`claude-sonnet-4-5`, `claude-haiku-4-5`).
- Children are spawned as `pi --mode json -p --no-session --model … --tools …` without `--no-extensions`, so they should load the permission system. How an `ask` resolves in a child with no UI is **unresolved and needs a test** (for example a child told to run `git push`).
- Zero-install option: `pi -p --model <id> --tools read,grep,find,ls "<review task>"`.
- Packages looked at: `pi-subagents` (11 MB, detached background runners), `@tintinweb/pi-subagents` (git-worktree isolation, but scheduled unattended agents), `pi-advisor-flow` (cheap Executor plus strong Advisor with automatic gates; folds cost into `/cost`).
- Catalog (`opencode` provider, 80 models, 8 free, per million tokens input/output): `gpt-6-luna` $0.10/$0.50 (current); scouts `deepseek-v4-flash` $0.14/$0.28, `gpt-5-nano` $0.05/$0.40, free `nemotron-3-ultra-free`; reviewers `claude-sonnet-5-5` and `gpt-6-sol` $2/$10, `deepseek-v4-pro` $1.74/$3.84, `kimi-k3` $3/$15; stronger `claude-opus-5-5` $4/$20, `gpt-6-astra` and `claude-fable-5-1` $10/$50. Rough cost of reviewing a ~20K-in/3K-out diff: about $0.07 on `claude-sonnet-5-5`, about $0.14 on `claude-opus-5-5`.

### Initial coding-role map for the 18-model allowlist (user-supplied; untested)

Treat these roles as hypotheses to test on the user's repositories, not a leaderboard. Published benchmarks may use different harnesses and reasoning settings, and OpenCode pricing may differ from direct-provider pricing. The research supplied for this note used placeholder “Source” labels without URLs, so vendor claims below have not been independently checked here.

| Role | Models and suggested use |
|---|---|
| Planning, architecture, high-stakes review | `claude-opus-5-5` for architecture, migrations, audits, and risky-change review; `kimi-k3` for broad repo planning and coordination; `gpt-6-astra` for the hardest design/debugging problems and second opinions; `grok-4.7` for long-running work with repeated self-checks. |
| Difficult implementation and long-horizon coding | `kimi-k2.7-code` for multi-file and long tasks; `gpt-5.3-codex` as a mainline implementation agent; `gpt-6.1-sol` as a less expensive general coding alternative to Astra; `glm-5.3` for complex agent-driven changes; `deepseek-v4-pro` for hard reasoning/implementation, subject to route verification; `minimax-m3` for long-context, tool-heavy refactors and debugging; `muse-spark-1.3` for agentic coding and spec-led implementation; `qwen3.6-plus` for practical tool-using coding; `gpt-5.6-terra` for balanced day-to-day implementation. |
| Fast, repetitive, cost-sensitive work | `gpt-5.3-codex-spark` for focused edits and quick iterations; `gpt-6-luna` for routine high-volume fixes, explanations, and tests; `glm-5.3-flash` for routine implementation and tests, escalating to GLM-5.3 if stuck; `deepseek-v4.1-flash` for bulk iterative coding and test generation; `qwen3.8-flash` for fast implementation and tests with large context. |

Suggested workflow: plan with Opus 5.5, Kimi K3, or Astra; implement with GPT-5.3 Codex, Kimi K2.7 Code, GLM-5.3, or GPT-6.1 Sol; use a fast model for broad test generation and routine passes; then review important diffs with Opus 5.5 or Astra. Check provider routing before relying on `deepseek-v4-pro`: DeepSeek reportedly routes its direct-API V4 Pro requests to V4.1 Flash, and it is unknown whether OpenCode does the same.

**Allowlist wrinkle:** the earlier `pi --list-models` output reportedly omitted `gpt-5.3-codex-spark`, which may explain a picker showing 17 entries for an 18-name allowlist. An allowlist can filter models Pi knows, but cannot register a model missing from Pi's model catalog; verify current catalog availability and provider route before changing configuration.

## Capability 8: task and plan tracking

**Decision:** keep the markdown queue for durable tasks; use Pi's official `plan-mode` example, customized, for read-only planning; no packages.

**Durable queue.** `docs/Queue.md` per `AGENTS.md`: one sentence per item, detail in project docs, completed items deleted. A file in the repo shows in `git diff` and enforces nothing by itself; the rules live in `AGENTS.md`.

**In-session todo list (`todo.ts` example): not installed, by decision.** A workflow still has to be designed that integrates an in-session checklist with the markdown queue: how session todos relate to queue items (one sentence each, detail in project docs, completed items deleted), whether the queue item or the session list is the source of truth, and how "Queue it" output and plan steps flow between the two. Until then the queue file stays the only durable list and plan mode's progress widget covers a single plan.

**Rejected packages.** `@groeponline/pi-missions` (durable queues, but state under `~/.pi/missions/` as JSON and SQLite), `pi-note` (per-session only), `@juicesharp/rpiv-todo` and `@diegopetrucci/pi-todo` (in-session checklists), `pi-goal`/`pi-goal-x` (goal-completion loops, not a backlog), `@plannotator/pi-extension` (41 MB, 7 dependencies, bash unrestricted during planning), `@bacnh85/pi-plan` (careful read-only gating and review loops, small community, overlaps the parked subagent work; an optional classifier would call an outside service), `@mjasnikovs/pi-task` (AGPL-3.0, unauthenticated remote-control web UI, unattended mode).

### Installed: Pi's `plan-mode` example (customized)

The upstream example was read in full: no network, no subprocess, no file writes, no `eval`. `/plan` or `Ctrl+Alt+P` toggles it, and `pi --plan` starts in it. The agent writes a numbered list under a `Plan:` header; execution tracks `[DONE:n]` markers; state persists through session resume. The `edit` and `write` tools are removed from the active tool list in plan mode (a hard lock); other active tools (web, `ask_user_question`, MCP) stay available. The `⏸ plan` footer status may not show in every powerline preset.

**Why it was changed.** The upstream bash guard only required a command to start with an allowlisted word and contain no denylisted word. The real function **verified** these as holes: `find … -delete`, `cat x; python3 -c "…write…"`, `ls && node -e "…write…"`, `curl … -o file`, `curl -X POST -d @file`, `git branch <name>`, `env`, `printenv`, `cat ~/.ssh/config`, `sed -n 'w file'`. The permission system returned `ask` for every one (**verified**). The injected instructions also named a `questionnaire` tool and a brave-search skill that do not exist here, and execution used steps cut to about 50 characters.

**Applied (items 1 to 7, plus an effort prompt).** Originals are kept as `.orig`; roughly 150 to 200 changed lines in `index.ts` and 350 in `utils.ts`.

| # | Change |
|---|---|
| 1 | Stale tool names replaced with `ask_user_question` and `web_search`/`fetch_content`; `questionnaire` removed from the tool list |
| 2 | Full step text, including multi-line sub-bullets (joined as `step: detail; detail`), is used when the plan is shown or executed; the 50-character form only drives the footer widget; older saved state still loads |
| 3 | The bash check splits chains, pipes, and `$(…)`, and every part must be allowlisted; backticks, subshells, heredocs, process substitution, arithmetic expansion, and unbalanced quotes fail closed |
| 4 | Removed from the allowlist: `curl`, `wget`, `env`, `printenv`, `awk`, `sed`, `less`, `more`, `top`, `htop`, `cargo`. Blocked arguments: `find -delete/-exec/-ok/-fprint`, `sort -o`, `tree -o`, `fd -x`, `rg --pre`, `tail -f`, `git --output/--ext-diff`, `date -s`, `npm audit fix`. `git branch` and `git remote` allow listing forms only. Redirects to `/dev/null` and `2>&1` are allowed (the upstream blocked them) |
| 5 | Planning prompt tailored: start from the goal, check external APIs with web tools, state assumptions, name files, end with validation steps, and "planning is not authorization" |
| 6 | New "Queue it" outcome: leaves plan mode and tells the agent not to implement, but to record the plan as one sentence in the queue file per the project's `AGENTS.md` (or stop if the project defines none) |
| 7 | When plan mode starts, a dialog asks which model should plan (before the first planning turn). "Keep current model" is first and Escape keeps it. Candidates are the ids in `PLANNING_MODEL_CANDIDATES` at the top of `index.ts` that exist in the registry (`claude-sonnet-5-5`, `gpt-6-sol`, `claude-opus-5-5`, `deepseek-v4-pro`, `kimi-k3`). The pre-plan model is restored when plan mode ends (Execute, Queue it, or toggle off) |
| 9 | A second dialog asks for the planning effort (thinking level), after the model choice. "Keep current effort" is first and Escape keeps it. The menu lists only levels the active model supports (`getSupportedThinkingLevels`), excludes `off` and the current level, and is skipped when there is no real choice (for example `kimi-k3` supports only `max`, `deepseek-v4-pro` only `high` and `max`, and a non-reasoning model has none). The pre-plan effort is restored with the model when plan mode ends: model first, then effort, because supported levels depend on the model. The footer status shows the planner model and effort while they differ from the originals |

**Not applied: item 8** (consult the permission service from plan mode). Not recommended: the permission allow list contains mutating commands such as `mkdir`, `touch`, `cp`, `git add`, and `cargo build`, so "allowed" is not "read-only".

**Testing.** **Verified** offline: 133 command cases (47 allow, 86 block) with 0 failures, a behavior comparison against the upstream function, a mock-host flow test covering the model prompt, all four outcomes, a failing `setModel`, and resume, and a second flow test (23 checks) for the effort dialog using the real per-model level maps from the catalog and Pi's own `getSupportedThinkingLevels`. **Untested:** the real dialogs and the real `pi.setModel` and `pi.setThinkingLevel` behavior.

**Caveats.**
- The bash check is regex-based and still only a guard against accidents; the permission rules are the real gate.
- Switching models resets the prompt cache.
- The model and effort prompts do not appear with the `--plan` flag or on resume.
- Pi clamps a requested effort to what the active model supports, so restoring an effort that the restored model lacks lands on the nearest supported level.
- If the model or effort is changed by hand during planning, ending plan mode restores the pre-plan values and overrides that change.

## Cross-cutting findings

- **Pin every package to an exact version** (`pi install npm:pkg@x.y.z`). Auto-update code paths exist (billion-context-pi had one on by default), and a pin skips them.
- **npm now blocks dependency install scripts** that are not allow-listed (seen for `tree-sitter-bash`). Review any approval individually.
- **Popularity is not safety.** The most downloaded package for a category is often the heaviest. Many packages ship plain TypeScript, so reading the shipped source is feasible and found real issues (auto-update, a third-party endpoint behind an opt-in feature, a compaction cancel).
- **Extensions that cancel or replace compaction cannot be combined.**
- **Check Pi's own examples and docs first.** MCP, subagents, plan mode, and a todo list are all built in or shipped as examples. Pi bundles `builtin:mcp`, `builtin:llama.cpp`, `builtin:codemode`, and `builtin:tool-search`; `-builtin:<name>` in settings disables one.
- **Third-party endpoints behind opt-in features:** `api.typesafe.ai` ("Jev") appears in `pi-warden`, `pi-powerline-footer` (`autoFollowUp`), and an optional classifier in `@bacnh85/pi-plan`. Keep these off.
- **Licenses:** `context-mode` is Elastic License 2.0 (source-available); `@mjasnikovs/pi-task` is AGPL-3.0.
- **Extension tool names need permission entries** or they fall to the default `ask`.
- **Model catalog:** the `opencode` provider lists 80 models with prices and context windows in `~/.pi/agent/models-store.json`.

## Operating notes

- **Editing permission rules:** copy to a scratch config, validate against the package's schema, test with the offline method above, then deploy with mode 600. Rules reload on session start.
- **Switch compression tool:** see capability 6. One active at a time.
- **Remove a package:** `pi remove npm:<name>`; check `settings.json` afterward.
- **Footer colors:** edit `theme.json`, then `/reload` or restart.
- **Pi theme:** `pi --use-theme <name>` previews one for a single run; the saved choice is the `theme` setting. A theme file in `~/.pi/agent/themes/` named like its `name` hot-reloads.
- **kitty theme:** `kitten themes` changes it; send `SIGUSR1` to kitty (or press `ctrl+shift+f5`) to reload the config.
- **Spend check:** `/session` in Pi, or sum per-message `usage.cost` in the session files under `~/.pi/agent/sessions/`.

## Providers: OpenCode Zen and Go

Verified 2026-10-06. Pi has two providers for the two services: `opencode` (Zen, pay per token, 82 models in the catalog) and `opencode-go` (Go, a subscription, 29 models). Credentials live in `~/.pi/agent/auth.json` (mode 600), one `api_key` entry per provider. Pi's docs map both to the single environment variable `OPENCODE_API_KEY`.

**Findings**
- Both entries hold the **same key** (compared by hash, never printed). Both providers accept it, which fits one account-level key that works against both services. If a separate Go key was expected, create it in the OpenCode console and replace the `opencode-go` entry.
- The endpoints differ: Zen is `https://opencode.ai/zen/v1`, Go is `https://opencode.ai/zen/go/v1`. Each refuses the other's exclusive models ("Model is unavailable"). Go-only ids include `mimo-v2.5`, `mimo-v2.5-pro`, `mimo-v2.6-flash`, `mimo-v2.6-pro`, `qwen3.7-plus`, `hy3`, `hy4-preview`, `longcat-2.0`, and the two `muse-spark-*-contributor` models; 19 ids exist on both.
- `gpt-6-luna` answered on both providers with identical usage (397 tokens in, 5 out, about $0.00004). A Go-only model (`mimo-v2.6-flash`) answered on Go through Pi.
- Zen returns 403 "Model access is disabled" for `gpt-5-nano`, so a model toggle in the Zen console is off for this account. This is a console setting, not a key problem.
- Pi prints a harmless startup warning, `No models match pattern "opencode/gpt-5.3-codex-spark"`. It comes from a Pi default list, not from settings.
- **`gpt-5.3-codex-spark` is visible in the Zen console but not usable through Pi (2026-10-06).** It is toggled on in the console ($1.75 in, $14 out, $0.175 cached, the same as `gpt-5.3-codex`). Pi's local catalog has no entry for it; Pi's website catalog lists it under the hyphenated id `gpt-5-3-codex-spark` (128K context), and Zen rejects that spelling ("Model is unavailable"). The dotted id `gpt-5.3-codex-spark` is the real one: OpenCode's docs give it the endpoint `https://opencode.ai/zen/v1/responses`, and Zen recognizes it (`/chat/completions` and `/messages` answer "Model does not support this protocol"). But a raw request to `/responses` returned 404 "Cannot find any route", while `gpt-5.3-codex` answered 200 on the same route, so it is unreachable from here for now. The `opencode/gpt-5.3-codex-spark` entry in `enabledModels` matches nothing and causes the startup warning, which also comes from a Pi default list. A custom `models.json` entry would only help once the route works.
- Pi's reported cost is computed from catalog prices, so it cannot show which plan was billed. Check the OpenCode console: Zen balance and usage versus the Go usage window.

**Method (reusable)**
- `pi auth check --provider <name> --json` reports that a credential exists. It does not prove the key works.
- A real test: `pi -p --no-session --no-tools --no-extensions --provider <name> --model <id> "Reply with exactly the word: ok" < /dev/null`. Close stdin with `< /dev/null` when not on a terminal; otherwise print mode waits forever. Add `--mode json` to read the provider, model, usage, and cost from the last assistant message.
- Raw requests to the Go endpoint need an `x-opencode-session` header (Pi adds it); without it the endpoint returns `MissingSessionID`.

## Global and project instructions (`AGENTS.md`)

**Status: deferred.** Splitting the instructions into a global file and per-project files needs to happen eventually, not now. Today only the project file `AGENTS.md` exists in this repo; no global file exists on the machine.

**How Pi loads context files** (**verified** in Pi's docs and its loader source):
- A global file at `~/.pi/agent/AGENTS.md` (or `AGENTS.override.md`, `CLAUDE.md`) applies in every project.
- Load order: the global file first, then each directory from the filesystem root down to the working directory, so the most specific file comes last. All of them are concatenated.
- Each directory contributes one file, by priority: `AGENTS.override.md`, `AGENTS.md`, `AGENTS.MD`, `CLAUDE.md`, `CLAUDE.MD`.
- There is no real override mechanism. `AGENTS.override.md` replaces the `AGENTS.md` in its own directory only and never suppresses the global file. A project file wins only by coming later and by saying so explicitly ("in this project, X replaces the global rule Y").
- A file at `~/Repos/AGENTS.md` would load for every repo under it. `~/Repos/rat/CLAUDE.md` loads as that project's file and would stack under a global one.
- Context files load even in untrusted projects, and every file costs tokens each session, so keep the global one lean. `/reload` picks up changes.

**Suggested split when it happens.**
- Global: discussing a design is not authorization to build it; a question is not a command; confirm before commit or push; state non-obvious assumptions; revert a failed fix before trying the next; the queue convention (file named per project); writing style; the machine note about `-j 4`.
- Per project (this repo): the Rust and Bevy validation commands, lint guidance, tool availability, the `name.rs` module style, the visual-testing rule, and "work on `master` only" (suits a solo project).
- Keep Rust rules per project, because other repos under `~/Repos` are not Rust.

## Vetting checklist for a candidate package

Download and read before installing: `npm view <pkg>` for metadata, then `npm pack <pkg>@<version>` and extract.

- Install-time lifecycle scripts (`preinstall`, `install`, `postinstall`, `prepare`) in the package and in its dependencies.
- Subprocess use (`child_process`, `exec`, `spawn`) and whether arguments are arrays or shell strings.
- `eval`, `new Function`, base64 or char-code obfuscation.
- Hardcoded network hosts, telemetry, analytics, and every opt-in feature that would send data to a third party.
- Self-update or self-install code paths (`autoUpdate`, `npm install` calls).
- Handlers that cancel or replace core flows (`session_before_compact`, `tool_call` blocks, tool overrides).
- Direct and peer dependencies, optional peers, and lockfile entries resolving to git, http, or file sources.
- Whether the published tarball matches the repository source and the shipped bundle contains no hosts or code absent from the source. Check for an npm provenance attestation (`npm view <pkg> dist.attestations`).
- License (source-available and AGPL terms).
- Opt-in features that read sensitive data (browser cookies, credential commands) and whether they default to off.
- Where state is written on disk.
- Load the shipped entry point with a mock `pi` object and the real config to confirm what it registers (handlers, tools, commands).
- Add permission entries for every tool it registers, pin the exact version, and record the decision here.

## Ideas from comparing with other agents (none decided)

From a comparison with Claude Code, oh-my-pi, and OpenCode (2026-10-03). Only the improvements are recorded here.

| Idea | Borrowed from | Notes |
|---|---|---|
| Guard against repeated identical tool calls: after about three in a row, ask before continuing | OpenCode (`doom_loop` permission) | Small own extension on the `tool_call` event; no package needed. Useful because the cheap executor model may loop |
| Step cap per agent, to bound cost when subagents exist | OpenCode (`steps` setting) | Belongs with the parked subagent design (capability 7) |
| Optional OS-level sandbox for unattended or untrusted work | Claude Code (sandboxing plus permissions as two layers) | Revisit only if Pi runs unattended or on a repo that is not trusted. Docker and `bwrap` are installed; the earlier decision to decline stands for now |
| Lock the permission config against edits by the agent | Claude Code (managed settings) | The root-owned lock described under capability 1, not applied |
| Edit-safety guard that rejects stale edits (read before edit, or content anchors) | oh-my-pi (hashline edits) | Evaluate only if live use shows failed or misplaced edits. The vendor benchmark is the author's own: best case 10x on one model, about 15 points on average |

## Open items

- **Global/project `AGENTS.md` split:** needed eventually, not now. See "Global and project instructions".
- **Ideas from the agent comparison:** decide which to adopt, if any; see the section above.
- **Ideas for improving the setup:** 65 researched ideas from other agents and real Pi configs, none decided; see `docs/pi-ideas.md`.
- **Capability 7:** research subagents and a custom workflow, including the permission-in-children test.
- **Observational memory versus billion-context-pi:** run the comparison and decide which stays.
- **Todo and queue workflow:** design how an in-session todo list (the `todo.ts` example, not installed) integrates with `docs/Queue.md`; see capability 8.
- **Pin-bump policy:** decide how and when to review and bump pinned versions.
- **Untested in a live session:** ask-user dialog, powerline rendering, plan-mode flow, observational memory compaction, MCP server permissions.

## Sources

Pi docs (installed with Pi under `/usr/lib/node_modules/pi/packages/coding-agent/docs/`, also at pi.dev/docs/latest): security, containerization, mcp, settings, compaction, usage. Package pages at [pi.dev/packages](https://pi.dev/packages), [Awesome Pi](https://awesome-pi.site/extensions/), [Composio's list](https://composio.dev/content/top-pi-extensions).
