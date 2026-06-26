# Development Harness

This document defines the expected local quality gates and fixtures. The actual commands will become active once the Rust workspace is scaffolded.

The current local harness uses Rust 1.87.

## Standard Commands

Use these commands:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p commitchi-tui -- --repo .
```

If the binary package name changes, update this file and `docs/HANDOFF.md`.

## Test Fixture Plan

Create small fixture repositories during tests with `tempfile` and local Git operations through `git2` where possible.

Fixture cases:

- Empty repository error.
- Linear history with 3 commits.
- File add/modify/delete.
- Rename or move.
- Binary file.
- Large diff that triggers truncation.
- Merge commit with two parents.

## Core Tests

`crates/core` should test:

- Repository discovery.
- Commit pagination order.
- Diff stats.
- Structured line classification.
- Large-diff truncation.
- Merge handling default.

## Pet Tests

`crates/pet` should test:

- Mood decay thresholds.
- Per-repo state load/save.
- Global state load/save.
- Scope behavior: repo, global, both.
- Reaction mapping from diff stats.

## TUI Tests

Keep most TUI logic testable without terminal rendering:

- Keybinding mapping.
- App state reducer.
- Playback speed changes.
- Animation progression.
- Timeline position math.

Snapshot tests can be added later with Ratatui buffers if useful.

## Manual Smoke Test

After Phase 1:

```sh
cd /path/to/a/git/repo
commitchi
```

Expected:

- TUI opens in alternate screen.
- Commit list/timeline appears.
- `h`/`l` and arrow keys move through commits.
- Diff and file list update.
- `q` exits cleanly and restores the terminal.

## CI Candidate

Once the workspace exists, add a basic CI job:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
