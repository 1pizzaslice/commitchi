# Roadmap

## Current Status

Phase 3 is complete on `phase/3-pet-persistence`. The TUI now supports pet mood persistence, repo/global state scopes, hook recording, hook installation, live state reload, and pet panel rendering. Phase 4 is pending explicit approval.

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

Status: pending.

Deliverables:

- Reaction heuristics based on current commit stats.
- Reaction text/sprite overlays.
- Playback-to-pet event wiring.

### Phase 5: Polish

Status: pending.

Deliverables:

- TOML config.
- README install instructions.
- More robust tests.
- Cross-platform notes.
- Release-oriented CLI help.

## Branch Strategy

- `main` is the stable checkpoint branch at phase boundaries.
- Active implementation should happen on branches named `phase/N-short-name`.
- Phase 1 should start from `main` on `phase/1-mvp-time-machine`.
- Merge or fast-forward back to `main` only when that phase's exit criteria are met.
