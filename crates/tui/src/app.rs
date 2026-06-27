use std::path::{Path, PathBuf};
use std::time::Duration;

use commitchi_core::{
    CommitSummary, DiffOptions, Error as CoreError, FileStatus, RepoHandle, StructuredDiff,
};
use commitchi_pet::{
    now_seconds, ActivityRecord, Mood, MoodConfig, PetScope, PetState, PetStateFiles, Reaction,
    ReactionStats,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use thiserror::Error;

use crate::animation::{AnimationConfig, DiffAnimation};
use crate::bindings::Command;

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Core(#[from] CoreError),

    #[error(transparent)]
    Pet(#[from] commitchi_pet::Error),
}

#[derive(Debug)]
pub struct App {
    repo: RepoHandle,
    commits: Vec<CommitSummary>,
    selected: usize,
    diff: StructuredDiff,
    diff_options: DiffOptions,
    diff_scroll: u16,
    animation_config: AnimationConfig,
    diff_animation: DiffAnimation,
    playing: bool,
    playback_progress: f64,
    pet_scope: PetScope,
    pet_mood_config: MoodConfig,
    pet_state_files: PetStateFiles,
    repo_pet_state: PetState,
    global_pet_state: PetState,
    pet_reaction: Reaction,
    tiny_commit_streak: usize,
    input_mode: InputMode,
    pet_elapsed: Duration,
}

/// Frame-timing state for the animated pet sprite, derived from a free-running
/// clock so the creature blinks, breathes, and cycles particles on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PetFrameState {
    pub blink: bool,
    pub particle_phase: usize,
    pub bob: usize,
}

/// Active text-entry mode layered over the normal keybindings.
#[derive(Debug, Clone, PartialEq, Eq)]
enum InputMode {
    Normal,
    Jump(JumpPrompt),
}

/// State for the interactive "jump to position or commit hash" prompt.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct JumpPrompt {
    buffer: String,
    error: Option<String>,
}

impl App {
    pub fn load(
        repo_path: impl AsRef<Path>,
        diff_options: DiffOptions,
        animation_config: AnimationConfig,
        pet_scope: PetScope,
        pet_mood_config: MoodConfig,
    ) -> Result<Self> {
        let repo = RepoHandle::discover(repo_path)?;
        let commits = repo.commit_summaries()?;
        if commits.is_empty() {
            return Err(CoreError::EmptyRepository.into());
        }

        let selected = 0;
        let diff = repo.diff_for_commit(&commits[selected].hash, diff_options)?;
        let diff_animation = DiffAnimation::new(count_diff_lines(&diff));
        let (pet_reaction, tiny_commit_streak) = reaction_for_diff(&diff, 0);
        let pet_state_files = PetStateFiles::for_git_dir(repo.git_dir(), pet_scope)?;
        let _ = pet_state_files.ensure_parent_dirs();
        let repo_pet_state = pet_state_files.load_repo_or_default()?;
        let global_pet_state = pet_state_files.load_global_or_default()?;

        Ok(Self {
            repo,
            commits,
            selected,
            diff,
            diff_options,
            diff_scroll: 0,
            animation_config,
            diff_animation,
            playing: false,
            playback_progress: 0.0,
            pet_scope,
            pet_mood_config,
            pet_state_files,
            repo_pet_state,
            global_pet_state,
            pet_reaction,
            tiny_commit_streak,
            input_mode: InputMode::Normal,
            pet_elapsed: Duration::ZERO,
        })
    }

    pub fn apply_command(&mut self, command: Command) -> Result<bool> {
        match command {
            Command::Quit => return Ok(true),
            Command::PreviousCommit => self.move_by_from_input(-1)?,
            Command::NextCommit => self.move_by_from_input(1)?,
            Command::JumpBackward => self.move_by_from_input(-10)?,
            Command::JumpForward => self.move_by_from_input(10)?,
            Command::FirstCommit => self.move_to_from_input(0)?,
            Command::LastCommit => {
                self.move_to_from_input(self.commits.len().saturating_sub(1))?;
            }
            Command::ScrollUp => {
                self.diff_scroll = self.diff_scroll.saturating_sub(1);
            }
            Command::ScrollDown => {
                self.diff_scroll = self.diff_scroll.saturating_add(1);
            }
            Command::TogglePlayback => {
                self.playing = !self.playing;
                self.playback_progress = 0.0;
            }
            Command::FasterPlayback => self.animation_config.increase_commit_speed(),
            Command::SlowerPlayback => self.animation_config.decrease_commit_speed(),
            Command::FasterReveal => self.animation_config.increase_line_speed(),
            Command::SlowerReveal => self.animation_config.decrease_line_speed(),
            Command::BeginJump => self.begin_jump(),
            Command::Noop => {}
        }

        Ok(false)
    }

    /// Whether the interactive jump prompt is currently capturing input.
    pub fn is_jumping(&self) -> bool {
        matches!(self.input_mode, InputMode::Jump(_))
    }

    /// Current jump buffer and optional error message, for rendering the prompt.
    pub fn jump_state(&self) -> Option<(&str, Option<&str>)> {
        match &self.input_mode {
            InputMode::Jump(prompt) => Some((prompt.buffer.as_str(), prompt.error.as_deref())),
            InputMode::Normal => None,
        }
    }

    /// Feed a raw key to the jump prompt. Returns `true` only on a quit request
    /// (Ctrl-C); Esc cancels the prompt and Enter commits the jump.
    pub fn handle_jump_key(&mut self, key: KeyEvent) -> Result<bool> {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Ok(true);
        }

        match key.code {
            KeyCode::Esc => self.input_mode = InputMode::Normal,
            KeyCode::Enter => self.commit_jump()?,
            KeyCode::Backspace => self.edit_jump(|buffer| {
                buffer.pop();
            }),
            KeyCode::Char(c) if !c.is_control() => self.edit_jump(|buffer| buffer.push(c)),
            _ => {}
        }

        Ok(false)
    }

    fn begin_jump(&mut self) {
        self.playing = false;
        self.playback_progress = 0.0;
        self.input_mode = InputMode::Jump(JumpPrompt::default());
    }

    fn edit_jump(&mut self, edit: impl FnOnce(&mut String)) {
        if let InputMode::Jump(prompt) = &mut self.input_mode {
            edit(&mut prompt.buffer);
            prompt.error = None;
        }
    }

    fn commit_jump(&mut self) -> Result<()> {
        let InputMode::Jump(prompt) = &self.input_mode else {
            return Ok(());
        };

        let query = prompt.buffer.clone();
        match resolve_jump(&query, &self.commits) {
            Ok(index) => {
                self.input_mode = InputMode::Normal;
                self.move_to_from_input(index)?;
            }
            Err(message) => {
                if let InputMode::Jump(prompt) = &mut self.input_mode {
                    prompt.error = Some(message);
                }
            }
        }

        Ok(())
    }

    pub fn tick(&mut self, elapsed: Duration) -> Result<()> {
        self.pet_elapsed = self.pet_elapsed.saturating_add(elapsed);
        self.advance_playback(elapsed)?;
        self.diff_animation
            .advance(elapsed, self.animation_config.lines_per_second());
        Ok(())
    }

    /// Current pet animation frame derived from the free-running clock: a short
    /// blink every few seconds, a slow breathing bob, and a cycling particle
    /// phase.
    pub fn pet_animation(&self) -> PetFrameState {
        let ms = self.pet_elapsed.as_millis();
        PetFrameState {
            blink: ms % 3_400 < 150,
            particle_phase: (ms / 400) as usize,
            bob: ((ms / 700) % 2) as usize,
        }
    }

    pub fn reload_pet_state(&mut self) -> Result<()> {
        self.repo_pet_state = self.pet_state_files.load_repo_or_default()?;
        self.global_pet_state = self.pet_state_files.load_global_or_default()?;
        Ok(())
    }

    pub fn selected_commit(&self) -> &CommitSummary {
        &self.commits[self.selected]
    }

    pub fn diff(&self) -> &StructuredDiff {
        &self.diff
    }

    pub fn diff_scroll(&self) -> u16 {
        self.diff_scroll
    }

    pub fn diff_reveal_progress(&self) -> (usize, usize) {
        (
            self.diff_animation.visible_lines(),
            self.diff_animation.total_lines(),
        )
    }

    pub fn animation_config(&self) -> AnimationConfig {
        self.animation_config
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    pub fn position(&self) -> (usize, usize) {
        (self.selected + 1, self.commits.len())
    }

    pub fn repo_root(&self) -> &Path {
        self.repo.root()
    }

    pub fn pet_status(&self) -> PetStatus {
        let now = now_seconds();
        PetStatus {
            scope: self.pet_scope,
            reaction: self.pet_reaction,
            repo_mood: self
                .pet_scope
                .includes_repo()
                .then(|| self.repo_pet_state.mood_at(now, self.pet_mood_config)),
            global_mood: self
                .pet_scope
                .includes_global()
                .then(|| self.global_pet_state.mood_at(now, self.pet_mood_config)),
            repo_last_activity: self.repo_pet_state.last_activity().cloned(),
            global_last_activity: self.global_pet_state.last_activity().cloned(),
        }
    }

    pub fn pet_watch_paths(&self) -> Vec<PathBuf> {
        self.pet_state_files.watch_paths()
    }

    fn advance_playback(&mut self, elapsed: Duration) -> Result<()> {
        if !self.playing {
            return Ok(());
        }

        let last_index = self.commits.len().saturating_sub(1);
        if self.selected >= last_index {
            self.playing = false;
            self.playback_progress = 0.0;
            return Ok(());
        }

        self.playback_progress +=
            elapsed.as_secs_f64() * self.animation_config.commits_per_second();
        while self.playback_progress >= 1.0 {
            if self.selected >= last_index {
                self.playing = false;
                self.playback_progress = 0.0;
                break;
            }

            self.move_to(self.selected + 1)?;
            self.playback_progress -= 1.0;
            if self.selected >= last_index {
                self.playing = false;
                self.playback_progress = 0.0;
                break;
            }
        }

        Ok(())
    }

    fn move_by_from_input(&mut self, delta: isize) -> Result<()> {
        self.playback_progress = 0.0;
        self.move_by(delta)
    }

    fn move_to_from_input(&mut self, next: usize) -> Result<()> {
        self.playback_progress = 0.0;
        self.move_to(next)
    }

    fn move_by(&mut self, delta: isize) -> Result<()> {
        let selected = self.selected as isize;
        let max = self.commits.len().saturating_sub(1) as isize;
        let next = selected.saturating_add(delta).clamp(0, max) as usize;
        self.move_to(next)
    }

    fn move_to(&mut self, next: usize) -> Result<()> {
        if next == self.selected {
            return Ok(());
        }

        let previous = self.selected;
        self.selected = next;
        self.diff_scroll = 0;
        self.diff = self
            .repo
            .diff_for_commit(&self.selected_commit().hash, self.diff_options)?;
        self.diff_animation.reset(count_diff_lines(&self.diff));
        self.refresh_pet_reaction(next == previous + 1);
        Ok(())
    }

    fn refresh_pet_reaction(&mut self, sequential_forward: bool) {
        let prior_streak = if sequential_forward {
            self.tiny_commit_streak
        } else {
            0
        };
        let (reaction, tiny_commit_streak) = reaction_for_diff(&self.diff, prior_streak);
        self.pet_reaction = reaction;
        self.tiny_commit_streak = tiny_commit_streak;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PetStatus {
    pub scope: PetScope,
    pub reaction: Reaction,
    pub repo_mood: Option<Mood>,
    pub global_mood: Option<Mood>,
    pub repo_last_activity: Option<ActivityRecord>,
    pub global_last_activity: Option<ActivityRecord>,
}

fn count_diff_lines(diff: &StructuredDiff) -> usize {
    diff.files.iter().map(|file| file.lines.len()).sum()
}

/// Resolve a jump query to a zero-based commit index.
///
/// A bare in-range number is treated as a 1-based timeline position. Anything
/// else (including a numeric value outside the timeline) is matched as a
/// case-insensitive commit-hash prefix.
fn resolve_jump(query: &str, commits: &[CommitSummary]) -> std::result::Result<usize, String> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Err("enter a position or commit hash".to_owned());
    }

    if trimmed.chars().all(|c| c.is_ascii_digit()) {
        if let Ok(position) = trimmed.parse::<usize>() {
            if (1..=commits.len()).contains(&position) {
                return Ok(position - 1);
            }
        }
        // A numeric value outside the timeline falls through to hash matching so
        // an all-decimal hash prefix can still be entered.
    }

    let needle = trimmed.to_ascii_lowercase();
    let matches = commits
        .iter()
        .enumerate()
        .filter(|(_, commit)| commit.hash.to_ascii_lowercase().starts_with(&needle))
        .map(|(index, _)| index)
        .collect::<Vec<_>>();

    match matches.as_slice() {
        [] => Err(format!("no commit or position matches '{trimmed}'")),
        [index] => Ok(*index),
        _ => Err(format!(
            "'{trimmed}' is ambiguous ({} commits)",
            matches.len()
        )),
    }
}

fn reaction_for_diff(diff: &StructuredDiff, prior_tiny_commit_streak: usize) -> (Reaction, usize) {
    let mut stats = ReactionStats {
        files_changed: diff.stats.files_changed,
        additions: diff.stats.additions,
        deletions: diff.stats.deletions,
        binary_files: diff.files.iter().filter(|file| file.binary).count(),
        renamed_files: diff
            .files
            .iter()
            .filter(|file| file.status == FileStatus::Renamed)
            .count(),
        truncated: diff.truncated,
        tiny_commit_streak: 0,
    };

    let tiny_commit_streak = if stats.is_tiny_commit() {
        prior_tiny_commit_streak.saturating_add(1)
    } else {
        0
    };
    stats.tiny_commit_streak = tiny_commit_streak;

    (Reaction::from_stats(stats), tiny_commit_streak)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use git2::{Oid, Repository, Signature};
    use tempfile::TempDir;

    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn type_jump(app: &mut App, text: &str) {
        for c in text.chars() {
            app.handle_jump_key(key(KeyCode::Char(c)))
                .expect("jump key");
        }
    }

    fn three_commit_app(fixture: &Fixture) -> App {
        fixture.write_file("story.txt", "one\n");
        fixture.commit("first");
        fixture.write_file("story.txt", "two\n");
        fixture.commit("second");
        fixture.write_file("story.txt", "three\n");
        fixture.commit("third");
        app_for_fixture(fixture, AnimationConfig::new(100.0, 1.0))
    }

    struct Fixture {
        _tmp: TempDir,
        repo: Repository,
    }

    impl Fixture {
        fn new() -> Self {
            let tmp = tempfile::tempdir().expect("tempdir");
            let repo = Repository::init(tmp.path()).expect("init repo");
            Self { _tmp: tmp, repo }
        }

        fn path(&self) -> &Path {
            self.repo.workdir().expect("workdir")
        }

        fn write_file(&self, path: &str, contents: &str) {
            fs::write(self.path().join(path), contents).expect("write file");
            self.repo
                .index()
                .expect("index")
                .add_path(Path::new(path))
                .expect("add path");
        }

        fn commit(&self, message: &str) -> Oid {
            let signature = Signature::now("Test User", "test@example.com").expect("signature");
            let mut index = self.repo.index().expect("index");
            index.write().expect("write index");
            let tree_id = index.write_tree().expect("write tree");
            let tree = self.repo.find_tree(tree_id).expect("tree");
            let parent = self
                .repo
                .head()
                .ok()
                .and_then(|head| head.target())
                .and_then(|oid| self.repo.find_commit(oid).ok());
            let parents = parent.iter().collect::<Vec<_>>();

            self.repo
                .commit(
                    Some("HEAD"),
                    &signature,
                    &signature,
                    message,
                    &tree,
                    &parents,
                )
                .expect("commit")
        }
    }

    fn app_for_fixture(fixture: &Fixture, animation_config: AnimationConfig) -> App {
        App::load(
            fixture.path(),
            DiffOptions::default(),
            animation_config,
            PetScope::Repo,
            MoodConfig::default(),
        )
        .expect("load app")
    }

    #[test]
    fn tick_reveals_diff_lines() {
        let fixture = Fixture::new();
        fixture.write_file("story.txt", "one\n");
        fixture.commit("first");
        let mut app = app_for_fixture(&fixture, AnimationConfig::new(10.0, 1.0));

        assert_eq!(app.diff_reveal_progress().0, 0);

        app.tick(Duration::from_millis(100)).expect("tick");

        assert_eq!(app.diff_reveal_progress().0, 1);
    }

    #[test]
    fn moving_commit_resets_reveal_animation() {
        let fixture = Fixture::new();
        fixture.write_file("story.txt", "one\n");
        fixture.commit("first");
        fixture.write_file("story.txt", "two\n");
        fixture.commit("second");
        let mut app = app_for_fixture(&fixture, AnimationConfig::new(100.0, 1.0));

        app.tick(Duration::from_secs(1)).expect("tick");
        assert!(app.diff_reveal_progress().0 > 0);

        app.apply_command(Command::NextCommit).expect("next commit");

        assert_eq!(app.position(), (2, 2));
        assert_eq!(app.diff_reveal_progress().0, 0);
    }

    #[test]
    fn playback_advances_by_commit_speed_and_stops_at_end() {
        let fixture = Fixture::new();
        fixture.write_file("story.txt", "one\n");
        fixture.commit("first");
        fixture.write_file("story.txt", "two\n");
        fixture.commit("second");
        fixture.write_file("story.txt", "three\n");
        fixture.commit("third");
        let mut app = app_for_fixture(&fixture, AnimationConfig::new(100.0, 2.0));

        app.apply_command(Command::TogglePlayback)
            .expect("toggle playback");
        app.tick(Duration::from_millis(500)).expect("tick");
        assert_eq!(app.position(), (2, 3));

        app.tick(Duration::from_millis(500)).expect("tick");

        assert_eq!(app.position(), (3, 3));
        assert!(!app.is_playing());
    }

    #[test]
    fn playback_updates_pet_reaction_for_tiny_commit_streaks() {
        let fixture = Fixture::new();
        fixture.write_file("story.txt", "one\n");
        fixture.commit("first");
        fixture.write_file("story.txt", "two\n");
        fixture.commit("second");
        fixture.write_file("story.txt", "three\n");
        fixture.commit("third");
        let mut app = app_for_fixture(&fixture, AnimationConfig::new(100.0, 2.0));

        assert_eq!(app.pet_reaction, Reaction::Calm);

        app.apply_command(Command::TogglePlayback)
            .expect("toggle playback");
        app.tick(Duration::from_millis(500)).expect("first tick");
        assert_eq!(app.position(), (2, 3));
        assert_eq!(app.pet_reaction, Reaction::Calm);

        app.tick(Duration::from_millis(500)).expect("second tick");

        assert_eq!(app.position(), (3, 3));
        assert_eq!(app.pet_reaction, Reaction::Nodding);
    }

    #[test]
    fn resolve_jump_matches_positions_then_hashes() {
        let fixture = Fixture::new();
        let app = three_commit_app(&fixture);

        assert_eq!(resolve_jump("1", &app.commits), Ok(0));
        assert_eq!(resolve_jump("  3 ", &app.commits), Ok(2));

        let hash = app.commits[1].hash.clone();
        assert_eq!(resolve_jump(&hash[..7], &app.commits), Ok(1));
        assert_eq!(resolve_jump(&hash.to_uppercase(), &app.commits), Ok(1));

        assert!(resolve_jump("", &app.commits).is_err());
        assert!(resolve_jump("999", &app.commits).is_err());
        assert!(resolve_jump("zzzz", &app.commits).is_err());
    }

    #[test]
    fn jump_to_timeline_position_moves_selection() {
        let fixture = Fixture::new();
        let mut app = three_commit_app(&fixture);

        app.apply_command(Command::BeginJump).expect("begin jump");
        assert!(app.is_jumping());

        type_jump(&mut app, "3");
        app.handle_jump_key(key(KeyCode::Enter)).expect("enter");

        assert!(!app.is_jumping());
        assert_eq!(app.position(), (3, 3));
    }

    #[test]
    fn jump_to_commit_hash_prefix_moves_selection() {
        let fixture = Fixture::new();
        let mut app = three_commit_app(&fixture);
        let prefix = app.commits[1].hash[..6].to_owned();

        app.apply_command(Command::BeginJump).expect("begin jump");
        type_jump(&mut app, &prefix);
        app.handle_jump_key(key(KeyCode::Enter)).expect("enter");

        assert!(!app.is_jumping());
        assert_eq!(app.position(), (2, 3));
    }

    #[test]
    fn jump_keeps_prompt_open_on_invalid_query() {
        let fixture = Fixture::new();
        let mut app = three_commit_app(&fixture);

        app.apply_command(Command::BeginJump).expect("begin jump");
        type_jump(&mut app, "nope");
        app.handle_jump_key(key(KeyCode::Enter)).expect("enter");

        assert!(app.is_jumping());
        assert!(app.jump_state().expect("prompt").1.is_some());
        assert_eq!(app.position(), (1, 3));

        app.handle_jump_key(key(KeyCode::Backspace))
            .expect("backspace");
        assert!(app.jump_state().expect("prompt").1.is_none());
    }

    #[test]
    fn jump_escape_cancels_without_moving() {
        let fixture = Fixture::new();
        let mut app = three_commit_app(&fixture);

        app.apply_command(Command::BeginJump).expect("begin jump");
        type_jump(&mut app, "2");
        app.handle_jump_key(key(KeyCode::Esc)).expect("escape");

        assert!(!app.is_jumping());
        assert_eq!(app.position(), (1, 3));
    }

    #[test]
    fn begin_jump_stops_playback() {
        let fixture = Fixture::new();
        let mut app = three_commit_app(&fixture);

        app.apply_command(Command::TogglePlayback).expect("play");
        assert!(app.is_playing());

        app.apply_command(Command::BeginJump).expect("begin jump");
        assert!(!app.is_playing());
    }

    #[test]
    fn renders_pet_panel_without_panicking() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let fixture = Fixture::new();
        let mut app = three_commit_app(&fixture);

        // Wide layout shows the animated pet; render across a few animation
        // phases so the blink, bob, and particle branches all execute.
        let mut wide = Terminal::new(TestBackend::new(96, 32)).expect("wide terminal");
        for _ in 0..4 {
            wide.draw(|frame| crate::ui::draw(frame, &app))
                .expect("draw");
            app.tick(Duration::from_millis(850)).expect("tick");
        }

        // Narrow layout hides the pet entirely and must still render.
        let mut narrow = Terminal::new(TestBackend::new(60, 20)).expect("narrow terminal");
        narrow
            .draw(|frame| crate::ui::draw(frame, &app))
            .expect("draw narrow");
    }
}
