# Handoff

## Project

Name: `commitchi`

Goal: Rust TUI for animated Git history playback with a persistent pet companion.

## Current State

Phase 0 planning docs exist. No Rust source has been created yet. The next work session should start with Phase 1 after confirming approval.

The workspace root `/home/anish/CODE01/kuchbhi` contains multiple unrelated projects. `commitchi/` is a new scoped project directory for this work.

The workspace root has an empty `.git` directory, but `git status` does not recognize it as a valid repository. Do not assume the workspace root is a repo.

## User Confirmed Decisions

- Pet mood should support both per-repo and global modes.
- Default pet scope should be per-repo unless config says otherwise.
- Keybindings should support both Vim-style and arrow/Page keys.
- Animation speed must be customizable.

## Recommended Next Step

Ask for approval to start Phase 1. On approval:

1. Create Rust workspace under `commitchi/`.
2. Add crates:
   - `crates/core`
   - `crates/pet`
   - `crates/tui`
3. Implement Phase 1 only:
   - repo open
   - commit summaries
   - static diff
   - file list
   - scrubber
   - navigation
4. Run the dev harness commands that are available.
5. Update this handoff file and stop for Phase 2 approval.

## Remote Setup

The intended remote is:

```sh
git@github.com:1pizzaslice/commitchi.git
```

The user asked to configure and push Phase 0 docs now. After that push, future work should use `main` as the stable phase-boundary branch.

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

## Phase Boundary Rule

The user explicitly requested phased delivery and check-ins at every phase boundary. Do not build multiple phases in one pass.
