# Roadmap

## Current Status

Phase 5 is complete on `phase/5-polish`. Commitchi now loads optional TOML config, has release-oriented help and README usage/install docs, and includes broader core fixture coverage for rename, binary, truncation, and merge behavior.

## Phase Checklist

### Phase 0: Research and Plan

Status: complete.

Completed:

- Compared `git2` and `gix`.
- Confirmed `ratatui` plus `crossterm`.
- Reviewed relevant prior art.
- Chose hook plus state file over a true daemon for MVP.
- Chose structured `git2` diffs over shelling out to `git diff`.
- Confirmed product defaults with user:
  - Pet supports per-repo and global modes.
  - Keybindings support both Vim and arrows.
  - Animation speed is configurable.
- Added persistent project docs.

Next step:

- Phase 0 has no remaining work.

### Phase 1: MVP Time Machine

Status: complete.

Deliverables:

- Rust workspace scaffold.
- `commitchi` binary.
- Open repo from cwd or `--repo`.
- Load commit summaries.
- Navigate oldest to newest.
- Static diff display.
- File list pane.
- Timeline/scrubber.
- Basic keybindings.
- Large diff truncation.
- Basic tests for core repository/diff behavior using fixture repos.

Completed:

- Added a Rust workspace with `commitchi-core`, `commitchi-pet`, and `commitchi-tui`.
- Implemented repo discovery from cwd or `--repo`.
- Implemented oldest-to-newest commit summaries.
- Implemented static structured diffs with file list, stats, and truncation.
- Implemented a Ratatui/Crossterm TUI with commit metadata, file pane, diff pane, and timeline scrubber.
- Implemented basic keybindings:
  - `h`/Left and `l`/Right for commit navigation.
  - `j`/PageDown and `k`/PageUp for larger timeline jumps.
  - Up/Down for diff scrolling.
  - `q`, Esc, and Ctrl-C to quit.
- Added core fixture-repo tests and TUI keybinding/timeline tests.

Exit criteria passed:

- `cargo test --workspace` passes.
- Running the TUI in a Git repo lets the user navigate commits and read static diffs.
- No pet UI yet.

### Phase 2: Diff Animation

Status: complete.

Deliverables:

- Configurable line reveal speed.
- Configurable playback commit speed.
- Play/pause and speed controls.
- Animation reset when moving commits.
- Render loop separated from input/tick events.

Completed:

- Added TUI animation state for line-by-line diff reveal.
- Added `--lines-per-second` and `--commits-per-second` CLI options with defaults of `30.0` and `1.0`.
- Added playback controls:
  - Space toggles play/pause.
  - `+`/`=` and `-` adjust commit playback speed.
  - `]` and `[` adjust line reveal speed.
- Reset diff reveal progress when changing commits manually or during playback.
- Split the main loop into explicit input, tick, and render events through `crates/tui/src/events.rs`.
- Added tests for animation progression, speed controls, event scheduling, key mappings, playback advancement, and animation reset.

Exit criteria passed:

- `cargo fmt --all -- --check` passes.
- `cargo clippy --workspace --all-targets -- -D warnings` passes.
- `cargo test --workspace` passes.
- Running the TUI in this repo animates diff lines and exits cleanly with `q`.

### Phase 3: Pet and Persistence

Status: complete.

Deliverables:

- Pet mood model.
- Repo-local and global state modes.
- JSON state persistence.
- `commitchi hook post-commit`.
- `commitchi install-hook`.
- File watching while TUI is open.
- Pet panel/sprite rendering.

Completed:

- Added `commitchi-pet` mood decay and recent-consistency behavior.
- Added JSON pet state with repo-local and global state file helpers.
- Added `--pet-scope repo|global|both` for TUI display scope, defaulting to repo.
- Added `commitchi hook post-commit`, defaulting to recording both repo and global state.
- Added `commitchi install-hook`, defaulting to a managed hook that records both scopes.
- Added `notify`-based state watching and reload while the TUI is open.
- Added a right-side pet sprite/status panel that renders at normal 80-column terminal width.
- Added tests for mood decay, persistence, scope behavior, hook block management, and existing TUI behavior.

Exit criteria passed:

- `cargo fmt --all -- --check` passes.
- `cargo clippy --workspace --all-targets -- -D warnings` passes.
- `cargo test --workspace` passes.
- Running the TUI in this repo animates diff lines, shows the pet panel, and exits cleanly with `q`.
- A temporary repo smoke test records `.git/commitchi/state.json` through `commitchi hook post-commit` and installs a managed `post-commit` hook through `commitchi install-hook`.

### Phase 4: The Merge

Status: complete.

Deliverables:

- Reaction heuristics based on current commit stats.
- Reaction text/sprite overlays.
- Playback-to-pet event wiring.

Completed:

- Added deterministic `ReactionStats` mapping in `commitchi-pet`.
- Added reactions for truncated or binary-only diffs, large rename-only diffs, large additions, large deletions, and tiny commit streaks.
- Wired the TUI to derive reaction stats from the selected commit's structured diff.
- Preserved a sequential tiny-commit streak during forward playback/navigation.
- Rendered reaction text and reaction-specific sprite faces in the pet panel.
- Added pet and TUI tests for reaction mapping and playback reaction updates.

Exit criteria passed:

- `cargo fmt --all -- --check` passes.
- `cargo clippy --workspace --all-targets -- -D warnings` passes.
- `cargo test --workspace` passes.
- Running the TUI in this repo shows the pet reaction overlay and exits cleanly with `q`.

### Phase 5: Polish

Status: complete.

Deliverables:

- TOML config.
- README install instructions.
- More robust tests.
- Cross-platform notes.
- Release-oriented CLI help.

Completed:

- Added `commitchi.toml` loading from the repository root, with `--config <FILE>` override.
- Added config support for pet scope, mood thresholds, animation speeds, and git diff limits.
- Preserved explicit CLI flags as the highest-priority overrides over config.
- Made `commitchi hook post-commit` and `commitchi install-hook` default to the configured pet scope unless `--scope` is passed.
- Added validation for missing explicit config, invalid TOML, non-positive speeds/limits, and unordered mood thresholds.
- Added README install, usage, config, pet state, hook, and cross-platform notes.
- Strengthened top-level CLI help and subcommand descriptions.
- Added core robustness tests for rename detection, binary diffs, file-list truncation, and first-parent merge diffs.

Exit criteria passed:

- `cargo fmt --all -- --check` passes.
- `cargo clippy --workspace --all-targets -- -D warnings` passes.
- `cargo test --workspace` passes.
- `cargo run -p commitchi-tui -- --help` renders release-oriented help.
- Running the TUI in this repo shows the pet panel/reaction overlay, animates diff lines, and exits cleanly with `q`.

## Branch Strategy

- `main` is the stable checkpoint branch at phase boundaries.
- Active implementation should happen on branches named `phase/N-short-name`.
- Phase 1 should start from `main` on `phase/1-mvp-time-machine`.
- Merge or fast-forward back to `main` only when that phase's exit criteria are met.
