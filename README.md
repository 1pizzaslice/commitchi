# Commitchi

Commitchi is a Rust TUI for replaying a local Git repository's history like a time machine, with an ASCII companion whose mood reflects commit activity and whose reactions respond to the diff currently on screen.

Phase 2 implements the animated time-machine TUI: repo discovery, commit summaries, structured diffs, file list, timeline scrubber, truncation, line-by-line diff reveal, and playback controls. Pet UI and persistence are planned for later phases.

## Run

```sh
cargo run -p commitchi-tui -- --repo .
```

Animation speeds can be set at startup:

```sh
cargo run -p commitchi-tui -- --repo . --lines-per-second 60 --commits-per-second 2
```

Inside the TUI:

- `h`/Left and `l`/Right navigate commits.
- `j`/PageDown and `k`/PageUp jump through the timeline.
- Up/Down scroll the diff pane.
- Space toggles play/pause.
- `+`/`=` and `-` adjust commit playback speed.
- `]` and `[` adjust line reveal speed.
- `q`, Esc, or Ctrl-C exits.

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
