# Commitchi

Commitchi is a Rust TUI for replaying a local Git repository's history like a time machine, with an ASCII companion whose mood reflects commit activity and whose reactions respond to the diff currently on screen.

Phase 3 implements the animated time-machine TUI plus pet mood persistence: repo discovery, commit summaries, structured diffs, file list, timeline scrubber, truncation, line-by-line diff reveal, playback controls, repo/global pet state, hook commands, live state reload, and a pet panel.

## Run

```sh
cargo run -p commitchi-tui -- --repo .
```

Animation speeds can be set at startup:

```sh
cargo run -p commitchi-tui -- --repo . --lines-per-second 60 --commits-per-second 2
```

Pet display scope defaults to repo-local state and can be changed at startup:

```sh
cargo run -p commitchi-tui -- --repo . --pet-scope both
```

Inside the TUI:

- `h`/Left and `l`/Right navigate commits.
- `j`/PageDown and `k`/PageUp jump through the timeline.
- Up/Down scroll the diff pane.
- Space toggles play/pause.
- `+`/`=` and `-` adjust commit playback speed.
- `]` and `[` adjust line reveal speed.
- `q`, Esc, or Ctrl-C exits.

Record the current HEAD commit in pet state:

```sh
cargo run -p commitchi-tui -- --repo . hook post-commit
```

Install a Git `post-commit` hook that invokes Commitchi:

```sh
cargo run -p commitchi-tui -- --repo . install-hook
```

## Check

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Start here:

- [Product requirements](docs/PRD.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Roadmap](docs/ROADMAP.md)
- [Development harness](docs/DEV_HARNESS.md)
- [Handoff](docs/HANDOFF.md)
