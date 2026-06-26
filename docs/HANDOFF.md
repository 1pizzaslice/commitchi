# Handoff

## Project

Name: `commitchi`

Goal: Rust TUI for animated Git history playback with a persistent pet companion.

## Current State

Phase 1 is complete on `phase/1-mvp-time-machine`.

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

Stop at the Phase 1 boundary and ask for explicit approval before starting Phase 2.

On Phase 2 approval:

1. Add diff animation state and controls only.
2. Add configurable line reveal speed and commit playback speed.
3. Separate render/input/tick handling enough to support animation.
4. Keep pet UI, persistence, hooks, config files, and file watching for later phases.
5. Run the dev harness commands.
6. Update this handoff file and stop for Phase 3 approval.

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
- Event loop: channel-driven tick/render/input events.
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

## Last Verification

Commands passed:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p commitchi-tui -- --repo .
```

The smoke run opened the TUI against this repo and exited cleanly with `q`.

## Phase Boundary Rule

The user explicitly requested phased delivery and check-ins at every phase boundary. Do not build multiple phases in one pass.
