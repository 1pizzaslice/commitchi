use std::path::Path;

use commitchi_core::{CommitSummary, DiffOptions, Error, RepoHandle, StructuredDiff};

use crate::bindings::Command;

#[derive(Debug)]
pub struct App {
    repo: RepoHandle,
    commits: Vec<CommitSummary>,
    selected: usize,
    diff: StructuredDiff,
    diff_options: DiffOptions,
    diff_scroll: u16,
}

impl App {
    pub fn load(repo_path: impl AsRef<Path>, diff_options: DiffOptions) -> Result<Self, Error> {
        let repo = RepoHandle::discover(repo_path)?;
        let commits = repo.commit_summaries()?;
        if commits.is_empty() {
            return Err(Error::EmptyRepository);
        }

        let selected = 0;
        let diff = repo.diff_for_commit(&commits[selected].hash, diff_options)?;

        Ok(Self {
            repo,
            commits,
            selected,
            diff,
            diff_options,
            diff_scroll: 0,
        })
    }

    pub fn apply_command(&mut self, command: Command) -> Result<bool, Error> {
        match command {
            Command::Quit => return Ok(true),
            Command::PreviousCommit => self.move_by(-1)?,
            Command::NextCommit => self.move_by(1)?,
            Command::JumpBackward => self.move_by(-10)?,
            Command::JumpForward => self.move_by(10)?,
            Command::FirstCommit => self.move_to(0)?,
            Command::LastCommit => self.move_to(self.commits.len().saturating_sub(1))?,
            Command::ScrollUp => {
                self.diff_scroll = self.diff_scroll.saturating_sub(1);
            }
            Command::ScrollDown => {
                self.diff_scroll = self.diff_scroll.saturating_add(1);
            }
            Command::Noop => {}
        }

        Ok(false)
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

    pub fn position(&self) -> (usize, usize) {
        (self.selected + 1, self.commits.len())
    }

    pub fn repo_root(&self) -> &Path {
        self.repo.root()
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
        Ok(())
    }
}
