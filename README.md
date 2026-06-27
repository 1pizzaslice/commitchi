# Commitchi 🐾

**Replay your Git history like a movie — with a pixel-art pet that reacts to every commit.**

Commitchi is a terminal app that scrubs through a repository's history, animating
each diff instead of dumping static `git log -p` output. Alongside it lives a
small pixel companion whose mood reflects how you've been committing and whose
expression reacts to the diff currently on screen — excited at big additions,
wincing at big deletions, sleepy when the repo's been quiet.

It reads **only local Git metadata**. No network, no GitHub, no telemetry.

---

## Install (Linux)

One command — no Rust toolchain required:

```sh
curl -fsSL https://raw.githubusercontent.com/1pizzaslice/commitchi/main/install.sh | sh
```

This downloads a prebuilt static binary from the latest
[GitHub Release](https://github.com/1pizzaslice/commitchi/releases) and installs
it to `~/.local/bin/commitchi`.

- Install somewhere else: `curl -fsSL …/install.sh | PREFIX=/usr/local/bin sh`
  (use `sudo sh` if that directory needs root).
- Pin a version: `curl -fsSL …/install.sh | VERSION=v0.1.0 sh`

Supported: `x86_64` and `aarch64` Linux. The binary is statically linked, so it
runs across distributions without extra system libraries.

> Prefer to build it yourself? See [From source](#from-source) below.

After installing, open any Git repository and run:

```sh
commitchi
```

---

## Prerequisites

Commitchi draws its pet as real pixel art using colored half-block characters.
For it to look right (and not show up as stray boxes or wrong colors), your
terminal needs two things:

1. **True color (24-bit) support.** The pet's colors are RGB. Most modern
   terminal emulators support this — kitty, Alacritty, WezTerm, foot, Konsole,
   GNOME Terminal, recent Windows Terminal. Check yours:

   ```sh
   echo "$COLORTERM"   # should print: truecolor (or 24bit)
   ```

   If it's empty, your terminal may still support it — try Commitchi and see. If
   colors look banded or wrong, set `COLORTERM=truecolor` in your shell config,
   or switch to one of the terminals above.

2. **A UTF-8 locale and a font with block/box-drawing glyphs.** The pet and the
   diff borders use characters like `▀ ▄ █ ─ │`. Virtually every modern
   monospace font includes these (DejaVu Sans Mono, JetBrains Mono, Fira Code,
   any Nerd Font). Make sure your locale is UTF-8:

   ```sh
   echo "$LANG"        # e.g. en_US.UTF-8
   ```

You also need:

- A graphical terminal emulator (the bare Linux TTY console can't show true
  color — use any terminal app inside your desktop).
- A Git repository with at least one commit.

Preview every pet expression without opening a repo (a quick way to confirm your
terminal renders it correctly):

```sh
commitchi pet-demo
```

If you see a cute orange creature with distinct faces, you're all set. If you see
boxes or odd symbols, revisit the font/locale notes above.

---

## Usage

Open the current repository:

```sh
commitchi
```

Open a different one:

```sh
commitchi --repo /path/to/repo
```

### Controls

| Keys | Action |
|---|---|
| `h` / `←` and `l` / `→` | Previous / next commit |
| `j` / `PgDn` and `k` / `PgUp` | Jump through the timeline |
| `Home` / `End` | First / last commit |
| `g` or `:` | Jump to a timeline position or commit hash |
| `↑` / `↓` | Scroll the diff |
| `Space` | Play / pause auto-playback |
| `+` / `-` | Commit playback speed |
| `]` / `[` | Diff reveal (typing) speed |
| `q` / `Esc` / `Ctrl-C` | Quit |

At the jump prompt (`g` / `:`), type a 1-based timeline position (e.g. `42`) or a
commit-hash prefix (e.g. `a1b2c3`), then `Enter` to go or `Esc` to cancel.

---

## The pet

The pet's face is chosen from two layers:

- A **reaction** to the commit on screen, from its diff stats: excited (large
  additions), wincing (large deletions), curious (rename runs), confused
  (binary-only or truncated diffs), happy (a run of tiny commits).
- When the diff is unremarkable, the face falls back to its persisted **mood**,
  which reflects how recently and consistently you commit and decays over time:
  happy → neutral → anxious → sad.

It blinks, breathes, and shows little emotion particles on its own.

### Keep its mood up to date

The pet's mood is recorded when you commit. Install a managed Git `post-commit`
hook so it updates automatically:

```sh
commitchi install-hook
```

Mood is stored per repository under `.git/commitchi/state.json`, and optionally
globally across all your repos. See [Pet state & hooks](docs/PRD.md) for the full
details, scopes, and platform paths.

---

## Configure

Commitchi reads an optional `commitchi.toml` from the repository root (override
with `--config <FILE>`). CLI flags always win over config.

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

Equivalent one-off overrides:

```sh
commitchi --lines-per-second 60 --commits-per-second 2 --pet-scope both
```

See `commitchi --help` for every flag.

---

## From source

Requires Rust 1.87+ and a C toolchain (`git2` builds a vendored `libgit2`).

```sh
# install straight from the repo
cargo install --git https://github.com/1pizzaslice/commitchi commitchi-tui

# or clone and build
git clone https://github.com/1pizzaslice/commitchi
cd commitchi
cargo run -p commitchi-tui -- --repo .
```

---

## Uninstall

```sh
rm ~/.local/bin/commitchi              # or wherever you installed it
rm -rf .git/commitchi                  # per-repo pet state (optional)
```

To remove the managed Git hook, delete the `# commitchi hook` block from
`.git/hooks/post-commit`.

---

## License

Dual-licensed under either [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at
your option.

## Project docs

- [Product requirements](docs/PRD.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Roadmap](docs/ROADMAP.md)
- [Handoff notes](docs/HANDOFF.md)
