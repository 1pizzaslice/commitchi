# Commitchi PRD

## Summary

Commitchi is a terminal application that lets a user scrub through a Git repository's commit history like a movie. It shows animated diffs instead of static `git log -p` output, and renders a small ASCII or Unicode companion whose mood reflects commit recency and consistency.

## Goals

- Replay local Git history offline from the `.git` directory.
- Make commit-to-commit transitions feel animated and watchable.
- Keep diff navigation practical for normal development work.
- Persist pet mood across sessions.
- Let the pet react to playback events using deterministic heuristics.

## Non-goals

- No GUI or web version.
- No GitHub, PR, issue, or remote integration.
- No LLM summaries or AI-derived commit analysis.
- No full daemon in the MVP unless hook plus state file proves insufficient.

## Confirmed Decisions

- Project name: `commitchi`.
- Pet mood scope: support both per-repo and global modes. Default should be per-repo. Global mode should aggregate activity across repos through the shared app data directory.
- Keybindings: support both Vim-style keys and arrow/Page keys.
- Animation speed: customizable. Provide reasonable defaults but make line reveal speed and commit playback speed configurable.

## Functional Requirements

### Mode 1: Time Machine

- Load a Git repo, defaulting to the current working directory.
- Build a navigable timeline from oldest to newest commit.
- Render a bottom scrubber with commit marks and the active position.
- Support single-step navigation with arrows and `h`/`l`.
- Support larger jumps with `j`/`k` and PageUp/PageDown.
- Show commit metadata, changed file list, and diff pane.
- Support play/pause, speed up, slow down, and jump to commit hash or timeline position.
- Animate diffs line-by-line or typewriter-style after Phase 1.
- Handle merge commits and large diffs without freezing.

### Mode 2: Pet

- Render a small sprite in a configurable corner or panel.
- Mood states: thriving, content, neutral, anxious, sulking.
- Mood derives from commit recency and consistency.
- Mood decays over wall-clock time without commits.
- Mood thresholds are configurable.
- State persists across sessions.
- State updates live when a commit happens while the TUI is open.
- Scope can be per-repo, global, or both displayed together.
- Until TOML config lands, the TUI display scope is controlled by `--pet-scope`, defaulting to `repo`. Hook recording defaults to `both` so global aggregation has data when enabled.

### Mode 3: The Merge

- During playback, the pet reacts to the current commit's diff stats.
- Example heuristics:
  - Large deletion streak: wince.
  - Large addition: excited.
  - Many tiny commits in sequence: content/nodding.
  - Large rename-only change: curious.
  - Binary-only or huge truncated diff: confused.
- Reactions are deterministic and based on structured diff stats.

## Configuration

Initial config should live in a TOML file, likely `commitchi.toml`.

Candidate fields:

```toml
[pet]
scope = "repo" # repo | global | both
position = "top-right"

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
merge_strategy = "first-parent"
```

## UX Principles

- First screen is the usable TUI, not a landing page.
- The diff pane is primary; the pet should add personality without stealing core navigation space.
- Degrade gracefully on narrow terminals.
- Never block the UI while computing a large diff.
