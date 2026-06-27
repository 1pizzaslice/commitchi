# Handoff

## Project

Name: `commitchi`

Goal: Rust TUI for animated Git history playback with a persistent pet companion.

## Current State

Phase 4 is complete on `phase/4-the-merge`.

The Rust workspace exists with:

- `crates/core` as `commitchi-core`
- `crates/pet` as `commitchi-pet`
- `crates/tui` as `commitchi-tui`, exposing the `commitchi` binary

The workspace root `/home/anish/CODE01/kuchbhi` contains multiple unrelated projects. `commitchi/` is a new scoped project directory for this work.

The project directory `/home/anish/CODE01/kuchbhi/commitchi` is a valid Git repository. Do not run remote Git operations unless the user explicitly asks in that turn.

## User Confirmed Decisions

- Pet mood should support both per-repo and global modes.
- Default pet scope should be per-repo unless config says otherwise.
- Keybindings should support both Vim-style and arrow/Page keys.
- Animation speed must be customizable.

## Recommended Next Step

Stop at the Phase 4 boundary and ask for explicit approval before starting Phase 5.

On Phase 5 approval:

1. Add TOML config loading for pet, animation, and git limits.
2. Add README install and usage instructions.
3. Strengthen release-oriented CLI help and cross-platform notes.
4. Add any remaining robustness tests called out by the harness.
5. Run the dev harness commands.
6. Update this handoff file and stop for final approval.

## Remote Setup

The configured remote is:

```sh
git@github.com:1pizzaslice/commitchi.git
```

Do not run remote Git operations, including `git push`, unless the user explicitly asks in that turn.

Branch strategy:

- Keep `main` as the checkpoint branch at phase boundaries.
- Use `phase/N-short-name` branches for active phase implementation.
- Phase 1 should start on `phase/1-mvp-time-machine`.
- Merge or fast-forward back to `main` only when the phase exit criteria pass.

## Research Decisions

- Git access: use `git2` for MVP.
- TUI: use `ratatui` plus `crossterm`.
- Event loop: separate tick/render/input event handling; channel-driven background work remains a future target.
- Persistence: JSON state file through `serde_json`.
- Config: TOML.
- Hook model: `post-commit` hook invoking `commitchi hook post-commit`.
- File watching: `notify`.

## Phase 1 Implementation Notes

- `commitchi-core` opens repos with `git2::Repository::discover`.
- Commit summaries are returned oldest-to-newest.
- Diffs use first-parent comparison for merge commits.
- Structured diff lines are built from `git2` callbacks, not parsed unified diff text.
- Large diffs are capped with configurable `line_limit` and `file_limit`.
- `commitchi-tui` currently loads diffs synchronously when changing commits.
- Keybindings:
  - `h`/Left and `l`/Right navigate one commit.
  - `j`/PageDown and `k`/PageUp jump by ten commits.
  - Up/Down scroll the diff pane.
  - Home/End jump to first/last commit.
  - `q`, Esc, and Ctrl-C quit.
- `commitchi-pet` is only scaffolded with basic domain enums/state; no pet UI or persistence exists yet.
- The local Rust toolchain is 1.87. `Cargo.lock` pins Ratatui's transitive `instability` dependency to `0.3.10` so clippy works on this toolchain.

## Phase 2 Implementation Notes

- Phase 2 was implemented on `phase/2-diff-animation`.
- `crates/tui/src/animation.rs` adds:
  - `AnimationConfig` for line reveal and commit playback speeds.
  - `DiffAnimation` for visible-line progress.
- `crates/tui/src/events.rs` separates input, tick, and render events with an in-process scheduler.
- `commitchi` accepts:
  - `--lines-per-second`
  - `--commits-per-second`
- Keybindings added:
  - Space toggles play/pause.
  - `+`/`=` and `-` adjust commit playback speed.
  - `]` and `[` adjust line reveal speed.
- Diff reveal resets whenever the selected commit changes manually or through playback.
- Playback stops automatically at the newest commit.
- Diff loading is still synchronous when changing commits.
- Pet UI, persistence, hooks, config files, and file watching are still deferred.

## Phase 3 Implementation Notes

- Phase 3 was implemented on `phase/3-pet-persistence`.
- `commitchi-pet` now owns:
  - `PetScope` (`repo`, `global`, `both`)
  - `Mood`, `MoodConfig`, and recent-consistency mood decay behavior
  - `ActivityRecord`
  - JSON `PetState`
  - `StateFile` and `PetStateFiles`
- Repo-local state path is `.git/commitchi/state.json` via `Repository::path()/commitchi/state.json`.
- Global state path checks `COMMITCHI_DATA_DIR`, then `XDG_DATA_HOME`, then platform app-data fallbacks.
- The TUI defaults to `--pet-scope repo`; `--pet-scope global` and `--pet-scope both` are supported.
- `commitchi hook post-commit` records the current HEAD commit and defaults to `--scope both`.
- `commitchi install-hook` writes or replaces a managed `# commitchi hook begin` / `# commitchi hook end` block and defaults to `--scope both`.
- Existing hook content outside the managed block is preserved, and Unix hooks are made executable.
- The TUI watches active pet state directories with `notify` and reloads pet state on file changes.
- TUI pet state directory creation is best-effort so read-only Git metadata does not prevent history playback.
- The pet panel renders in the right body column at normal 80-column terminal width and is hidden on narrower layouts.
- Phase 3 deferred structured-diff reaction heuristics to Phase 4.

## Phase 4 Implementation Notes

- Phase 4 was implemented on `phase/4-the-merge`.
- `commitchi-pet` now owns deterministic `ReactionStats` classification.
- Reaction outcomes:
  - `confused` for truncated or binary-only diffs.
  - `curious` for rename-only changes touching at least three files.
  - `excited` for large addition-heavy diffs.
  - `wincing` for large deletion-heavy diffs.
  - `nodding` for three tiny commits in a sequential playback/navigation streak.
  - `calm` for unremarkable diffs.
- `commitchi-tui` maps the selected commit's `StructuredDiff` into reaction stats and exposes the current reaction through `PetStatus`.
- Forward playback and one-step forward navigation preserve the tiny-commit streak; jumps and backward navigation reset it.
- The pet panel now renders a short reaction message and a reaction-specific face above the persisted mood/status lines.
- Mood persistence, hook behavior, and state file format remain unchanged.

## Last Verification

Commands passed:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p commitchi-tui -- --repo .
```

The smoke run opened the TUI against this repo, showed the pet panel at 80 columns, rendered the `large addition` reaction overlay with the excited face, animated diff lines, and exited cleanly with `q`.

## Phase Boundary Rule

The user explicitly requested phased delivery and check-ins at every phase boundary. Do not build multiple phases in one pass.
