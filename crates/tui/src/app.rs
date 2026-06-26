use std::path::Path;
use std::time::Duration;

use commitchi_core::{CommitSummary, DiffOptions, Error, RepoHandle, StructuredDiff};

use crate::animation::{AnimationConfig, DiffAnimation};
use crate::bindings::Command;

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
}

impl App {
    pub fn load(
        repo_path: impl AsRef<Path>,
        diff_options: DiffOptions,
        animation_config: AnimationConfig,
    ) -> Result<Self, Error> {
        let repo = RepoHandle::discover(repo_path)?;
        let commits = repo.commit_summaries()?;
        if commits.is_empty() {
            return Err(Error::EmptyRepository);
        }

        let selected = 0;
        let diff = repo.diff_for_commit(&commits[selected].hash, diff_options)?;
        let diff_animation = DiffAnimation::new(count_diff_lines(&diff));

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
        })
    }

    pub fn apply_command(&mut self, command: Command) -> Result<bool, Error> {
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
            Command::Noop => {}
        }

        Ok(false)
    }

    pub fn tick(&mut self, elapsed: Duration) -> Result<(), Error> {
        self.advance_playback(elapsed)?;
        self.diff_animation
            .advance(elapsed, self.animation_config.lines_per_second());
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

    fn advance_playback(&mut self, elapsed: Duration) -> Result<(), Error> {
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

    fn move_by_from_input(&mut self, delta: isize) -> Result<(), Error> {
        self.playback_progress = 0.0;
        self.move_by(delta)
    }

    fn move_to_from_input(&mut self, next: usize) -> Result<(), Error> {
        self.playback_progress = 0.0;
        self.move_to(next)
    }

    fn move_by(&mut self, delta: isize) -> Result<(), Error> {
        let selected = self.selected as isize;
        let max = self.commits.len().saturating_sub(1) as isize;
        let next = selected.saturating_add(delta).clamp(0, max) as usize;
        self.move_to(next)
    }

    fn move_to(&mut self, next: usize) -> Result<(), Error> {
        if next == self.selected {
            return Ok(());
        }

        self.selected = next;
        self.diff_scroll = 0;
        self.diff = self
            .repo
            .diff_for_commit(&self.selected_commit().hash, self.diff_options)?;
        self.diff_animation.reset(count_diff_lines(&self.diff));
        Ok(())
    }
}

fn count_diff_lines(diff: &StructuredDiff) -> usize {
    diff.files.iter().map(|file| file.lines.len()).sum()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use git2::{Oid, Repository, Signature};
    use tempfile::TempDir;

    use super::*;

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
        App::load(fixture.path(), DiffOptions::default(), animation_config).expect("load app")
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
}
