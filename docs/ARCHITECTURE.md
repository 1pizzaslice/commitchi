# Architecture

## Recommended Stack

- Language: Rust.
- Local MSRV: Rust 1.87 for the current harness.
- Git access: `git2` for the MVP.
- TUI: `ratatui` with `crossterm`.
- Async/event loop: likely `tokio`, with a channel-driven event loop.
- File watching: `notify`, with polling fallback if native watching fails.
- Persistence: `serde` plus `serde_json` for pet state, `toml` for config.
- CLI: `clap`.
- Errors: `thiserror` for libraries, `color-eyre` or `miette` for the TUI binary.
- Time: `time` or `chrono`; prefer `time` unless crate ergonomics push otherwise.

## Git Layer Decision

Use `git2` first because it directly exposes the API surface needed for Phase 1 and Phase 2:

- Repository discovery/opening.
- Revwalk.
- Commit lookup.
- Tree-to-tree diffing.
- Diff stats.
- File deltas, hunks, and line callbacks.

Tradeoff: `git2` wraps libgit2 and brings a native dependency. This is acceptable for the MVP because the API is mature and maps cleanly to structured diff animation.

`gix` remains a possible future backend. It is pure Rust and active, but the broader workspace still carries more API churn risk for this app's first pass. Keep the public core API independent from `git2` so a later `gix` backend is possible.

## Workspace Layout

```text
commitchi/
  Cargo.toml
  crates/
    core/
    pet/
    tui/
  hooks/
    post-commit
  docs/
```

## Crate Responsibilities

### `crates/core`

No UI dependencies.

Responsibilities:

- Open and validate Git repositories.
- Resolve repository identity and root path.
- Walk commits lazily.
- Return commit metadata.
- Produce structured diffs.
- Summarize large diffs.
- Cache recently viewed diffs.

Main types:

```rust
pub struct RepoHandle;
pub struct CommitSummary;
pub struct CommitPage;
pub struct StructuredDiff;
pub struct FileDiff;
pub struct DiffLine;
pub struct DiffStats;
```

### `crates/pet`

No UI dependencies.

Responsibilities:

- Represent pet mood and reactions.
- Record commit events from hooks.
- Load/save state.
- Compute mood decay from current time.
- Support per-repo and global scopes.

Main types:

```rust
pub enum PetScope;
pub enum Mood;
pub enum Reaction;
pub struct PetState;
pub struct MoodConfig;
pub struct ActivityRecord;
```

### `crates/tui`

Binary crate.

Responsibilities:

- CLI parsing.
- Terminal lifecycle.
- Event loop.
- Keybindings.
- Rendering.
- Playback.
- Animation state.
- Watching pet state file for changes.

Main areas:

- `app.rs`: app state and commands.
- `events.rs`: input, tick, render, file-watch events.
- `ui.rs`: Ratatui rendering functions.
- `animation.rs`: diff reveal state.
- `bindings.rs`: key mapping.
- `config.rs`: config loading and defaults.

## Event Loop

Long-term, use a channel-driven loop with separate event kinds:

```rust
enum Event {
    Init,
    Input(KeyEvent),
    Tick,
    Render,
    StateFileChanged,
    DiffLoaded(Result<StructuredDiff, Error>),
    Quit,
}
```

Recommended defaults:

- Render rate: 30 or 60 FPS.
- Tick rate: 4 to 10 TPS for non-render state updates.
- Diff reveal: configurable lines per second.
- Commit playback: configurable commits per second.

Diff computation should happen off the render path. For Phase 1 it can be synchronous if fast enough, but the API should be shaped so Phase 2 can move work to a background task.

Phase 2 implementation note: the TUI currently uses an in-process `EventSchedule` instead of worker threads or channels. It still separates `Input`, `Tick`, and `Render` handling so animation state can advance independently from key handling and terminal drawing. Diff loading remains synchronous when changing commits; background diff loading is still future work.

## Diff Model

Do not parse unified diff text for the main path. Use `git2` diff callbacks to build structured lines:

```rust
enum DiffLineKind {
    Context,
    Addition,
    Deletion,
    HunkHeader,
    FileHeader,
    Binary,
    Truncated,
}
```

This gives the TUI stable data for:

- Colorizing lines.
- Progressive reveal.
- Diff stat heuristics.
- Large-diff truncation.
- File-pane summaries.

## Pet State

Repo-local state should be stored under `.git/commitchi/state.json` or another file inside Git metadata, not committed source.

Global state should be stored using platform data dirs, for example:

```text
~/.local/share/commitchi/state.json
```

The hook should call:

```sh
commitchi hook post-commit
```

The Rust command records the commit in repo-local state and optionally global state depending on config.

## Hook Strategy

Use a `post-commit` hook for MVP. It is simple, has the right semantics, and avoids a long-running daemon.

`commitchi install-hook` should:

- Detect the current Git repo.
- Create or update `.git/hooks/post-commit`.
- Preserve existing hook content by chaining where possible or writing a clear managed block.
- Make the hook executable on Unix.

## Large Repos

Initial strategy:

- Load commit metadata in pages.
- Cache recent commit summaries.
- Cache recent structured diffs using an LRU.
- Hard cap rendered diff lines and file count.
- Show summary rows when truncating.

Future strategy:

- Use commit graph/cache if needed.
- Preload nearby commits during playback.
- Add branch filtering and first-parent mode.

## Phase 1 Implementation Notes

- Workspace package names:
  - `commitchi-core`
  - `commitchi-pet`
  - `commitchi-tui`
- The `commitchi-tui` package exposes the `commitchi` binary.
- Phase 1 diff loading is synchronous, but app state calls through `RepoHandle::diff_for_commit` so Phase 2 can move diff work off the render path without changing the core model.
- `commitchi-core` uses `git2` diff callbacks to build `StructuredDiff`, `FileDiff`, and `DiffLine` values directly.
- The lockfile pins Ratatui's transitive `instability` dependency to `0.3.10`, which is compatible with Rust 1.87.

## Phase 2 Implementation Notes

- `crates/tui/src/animation.rs` owns `AnimationConfig` and `DiffAnimation`.
- Diff reveal starts at zero visible lines and advances on tick events according to `lines_per_second`.
- Commit playback advances on tick events according to `commits_per_second`, stops at the newest commit, and resets the reveal animation for each commit.
- Phase 2 speed configuration is exposed through CLI flags only:
  - `--lines-per-second`
  - `--commits-per-second`
- TOML config loading is still reserved for Phase 5.
- `crates/tui/src/events.rs` provides the current input/tick/render event scheduler.
- Pet UI, persistence, hooks, config files, and file watching remain out of scope until later phases.
