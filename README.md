# Commitchi

Commitchi is a Rust TUI for replaying a local Git repository's history like a time machine, with an ASCII companion whose mood reflects commit activity and whose reactions respond to the diff currently on screen.

Commitchi reads only local Git metadata. It has no GitHub, remote, or network integration.

## Install

Requirements:

- Rust 1.87 or newer.
- A terminal with alternate-screen support.
- A local Git repository with at least one commit.

Install the current workspace build:

```sh
cargo install --path crates/tui
```

Or run without installing:

```sh
cargo run -p commitchi-tui -- --repo .
```

## Usage

Open the current repository:

```sh
commitchi
```

Open another repository:

```sh
commitchi --repo /path/to/repo
```

Inside the TUI:

- `h`/Left and `l`/Right navigate commits.
- `j`/PageDown and `k`/PageUp jump through the timeline.
- Home/End jump to first/last commit.
- Up/Down scroll the diff pane.
- Space toggles play/pause.
- `+`/`=` and `-` adjust commit playback speed.
- `]` and `[` adjust line reveal speed.
- `q`, Esc, or Ctrl-C exits.

## Config

Commitchi reads `commitchi.toml` from the repository root by default. Use `--config <FILE>` to choose another config file. CLI flags override config values.

Example:

```toml
[pet]
scope = "repo" # repo | global | both

[pet.thresholds]
thriving_hours = 24
content_hours = 48
neutral_hours = 96
anxious_hours = 168

[animation]
lines_per_second = 30
commits_per_second = 1

[git]
large_diff_line_limit = 2000
large_diff_file_limit = 100
```

Equivalent startup overrides:

```sh
commitchi --lines-per-second 60 --commits-per-second 2 --pet-scope both
```

## Pet State

Repo-local pet state is stored under `.git/commitchi/state.json`. Global pet state uses `COMMITCHI_DATA_DIR/state.json` when set, then platform app-data defaults such as:

- Linux: `$XDG_DATA_HOME/commitchi/state.json` or `~/.local/share/commitchi/state.json`.
- macOS: `~/Library/Application Support/commitchi/state.json`.
- Windows: `%APPDATA%\commitchi\state.json`.

Record the current HEAD commit in pet state:

```sh
commitchi hook post-commit
```

Install a managed Git `post-commit` hook:

```sh
commitchi install-hook
```

Hook commands use `[pet].scope` from config unless `--scope repo|global|both` is provided:

```sh
commitchi install-hook --scope both
```

On Unix-like systems Commitchi marks the hook executable. On Windows, Git still executes hooks through its shell environment, so `commitchi` must be on `PATH` for the hook to record activity.

## CLI Help

Show top-level help:

```sh
commitchi --help
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
