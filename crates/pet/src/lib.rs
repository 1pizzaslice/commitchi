//! Pet mood, activity state, and JSON persistence.

use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

const STATE_VERSION: u32 = 1;
const MAX_ACTIVITY_RECORDS: usize = 512;
const RECENT_WINDOW_HOURS: i64 = 7 * 24;
const CONSISTENT_RECENT_COMMITS: usize = 3;

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to read or write pet state at {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to parse pet state at {path}: {source}")]
    Json {
        path: PathBuf,
        source: serde_json::Error,
    },

    #[error("could not determine an app data directory for global pet state")]
    MissingDataDirectory,

    #[error("state target is not configured for {0} scope")]
    MissingStateTarget(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PetScope {
    Repo,
    Global,
    Both,
}

impl PetScope {
    pub fn includes_repo(self) -> bool {
        matches!(self, Self::Repo | Self::Both)
    }

    pub fn includes_global(self) -> bool {
        matches!(self, Self::Global | Self::Both)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Repo => "repo",
            Self::Global => "global",
            Self::Both => "both",
        }
    }
}

impl Default for PetScope {
    fn default() -> Self {
        Self::Repo
    }
}

impl fmt::Display for PetScope {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for PetScope {
    type Err = ParsePetScopeError;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value {
            "repo" => Ok(Self::Repo),
            "global" => Ok(Self::Global),
            "both" => Ok(Self::Both),
            _ => Err(ParsePetScopeError(value.to_owned())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsePetScopeError(String);

impl fmt::Display for ParsePetScopeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid pet scope '{}'; expected repo, global, or both",
            self.0
        )
    }
}

impl std::error::Error for ParsePetScopeError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Mood {
    Thriving,
    Content,
    Neutral,
    Anxious,
    Sulking,
}

impl Mood {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Thriving => "thriving",
            Self::Content => "content",
            Self::Neutral => "neutral",
            Self::Anxious => "anxious",
            Self::Sulking => "sulking",
        }
    }
}

impl fmt::Display for Mood {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Reaction {
    Calm,
    Excited,
    Curious,
    Confused,
    Wincing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoodConfig {
    pub thriving_hours: i64,
    pub content_hours: i64,
    pub neutral_hours: i64,
    pub anxious_hours: i64,
}

impl Default for MoodConfig {
    fn default() -> Self {
        Self {
            thriving_hours: 24,
            content_hours: 48,
            neutral_hours: 96,
            anxious_hours: 168,
        }
    }
}

impl MoodConfig {
    pub fn mood_for_hours_since_commit(self, hours: i64) -> Mood {
        if hours <= self.thriving_hours {
            Mood::Thriving
        } else if hours <= self.content_hours {
            Mood::Content
        } else if hours <= self.neutral_hours {
            Mood::Neutral
        } else if hours <= self.anxious_hours {
            Mood::Anxious
        } else {
            Mood::Sulking
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivityRecord {
    pub repo_id: String,
    pub commit_hash: String,
    pub commit_time_seconds: i64,
    pub recorded_at_seconds: i64,
}

impl ActivityRecord {
    pub fn new(
        repo_id: impl Into<String>,
        commit_hash: impl Into<String>,
        commit_time_seconds: i64,
        recorded_at_seconds: i64,
    ) -> Self {
        Self {
            repo_id: repo_id.into(),
            commit_hash: commit_hash.into(),
            commit_time_seconds,
            recorded_at_seconds,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PetState {
    version: u32,
    records: Vec<ActivityRecord>,
}

impl PetState {
    pub fn new() -> Self {
        Self {
            version: STATE_VERSION,
            records: Vec::new(),
        }
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn records(&self) -> &[ActivityRecord] {
        &self.records
    }

    pub fn record_commit(&mut self, record: ActivityRecord) {
        self.records.retain(|existing| {
            existing.repo_id != record.repo_id || existing.commit_hash != record.commit_hash
        });
        self.records.push(record);
        self.records
            .sort_by_key(|record| record.recorded_at_seconds);

        let overflow = self.records.len().saturating_sub(MAX_ACTIVITY_RECORDS);
        if overflow > 0 {
            self.records.drain(0..overflow);
        }
    }

    pub fn last_activity(&self) -> Option<&ActivityRecord> {
        self.records
            .iter()
            .max_by_key(|record| record.recorded_at_seconds)
    }

    pub fn mood_at(&self, now_seconds: i64, config: MoodConfig) -> Mood {
        let Some(last_activity) = self.last_activity() else {
            return Mood::Neutral;
        };

        let seconds_since = now_seconds
            .saturating_sub(last_activity.recorded_at_seconds)
            .max(0);
        let hours_since = seconds_since / 3_600;
        let base_mood = config.mood_for_hours_since_commit(hours_since);

        if matches!(base_mood, Mood::Neutral | Mood::Anxious) {
            let recent_count = self
                .records
                .iter()
                .filter(|record| {
                    let age_hours = now_seconds
                        .saturating_sub(record.recorded_at_seconds)
                        .max(0)
                        / 3_600;
                    age_hours <= RECENT_WINDOW_HOURS
                })
                .count();

            if recent_count >= CONSISTENT_RECENT_COMMITS {
                return match base_mood {
                    Mood::Neutral => Mood::Content,
                    Mood::Anxious => Mood::Neutral,
                    mood => mood,
                };
            }
        }

        base_mood
    }
}

impl Default for PetState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateFile {
    path: PathBuf,
}

impl StateFile {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn ensure_parent_dir(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|source| Error::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        Ok(())
    }

    pub fn load(&self) -> Result<PetState> {
        let contents = fs::read_to_string(&self.path).map_err(|source| Error::Io {
            path: self.path.clone(),
            source,
        })?;
        serde_json::from_str(&contents).map_err(|source| Error::Json {
            path: self.path.clone(),
            source,
        })
    }

    pub fn load_or_default(&self) -> Result<PetState> {
        match self.load() {
            Ok(state) => Ok(state),
            Err(Error::Io { source, .. }) if source.kind() == std::io::ErrorKind::NotFound => {
                Ok(PetState::default())
            }
            Err(err) => Err(err),
        }
    }

    pub fn save(&self, state: &PetState) -> Result<()> {
        self.ensure_parent_dir()?;
        let contents = serde_json::to_string_pretty(state).map_err(|source| Error::Json {
            path: self.path.clone(),
            source,
        })?;
        let tmp_path = self.path.with_extension("json.tmp");
        fs::write(&tmp_path, contents).map_err(|source| Error::Io {
            path: tmp_path.clone(),
            source,
        })?;
        fs::rename(&tmp_path, &self.path).map_err(|source| Error::Io {
            path: self.path.clone(),
            source,
        })?;

        Ok(())
    }

    pub fn record_commit(&self, record: ActivityRecord) -> Result<()> {
        let mut state = self.load_or_default()?;
        state.record_commit(record);
        self.save(&state)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PetStateFiles {
    repo: Option<StateFile>,
    global: Option<StateFile>,
}

impl PetStateFiles {
    pub fn for_git_dir(git_dir: impl AsRef<Path>, scope: PetScope) -> Result<Self> {
        let repo = scope
            .includes_repo()
            .then(|| StateFile::new(repo_state_path(git_dir.as_ref())));
        let global = if scope.includes_global() {
            Some(StateFile::new(global_state_path()?))
        } else {
            None
        };

        Ok(Self { repo, global })
    }

    pub fn from_paths(repo: Option<PathBuf>, global: Option<PathBuf>) -> Self {
        Self {
            repo: repo.map(StateFile::new),
            global: global.map(StateFile::new),
        }
    }

    pub fn repo(&self) -> Option<&StateFile> {
        self.repo.as_ref()
    }

    pub fn global(&self) -> Option<&StateFile> {
        self.global.as_ref()
    }

    pub fn ensure_parent_dirs(&self) -> Result<()> {
        if let Some(repo) = &self.repo {
            repo.ensure_parent_dir()?;
        }
        if let Some(global) = &self.global {
            global.ensure_parent_dir()?;
        }

        Ok(())
    }

    pub fn watch_paths(&self) -> Vec<PathBuf> {
        self.repo
            .iter()
            .chain(self.global.iter())
            .map(|file| file.path().to_path_buf())
            .collect()
    }

    pub fn load_repo_or_default(&self) -> Result<PetState> {
        self.repo
            .as_ref()
            .map(StateFile::load_or_default)
            .unwrap_or_else(|| Ok(PetState::default()))
    }

    pub fn load_global_or_default(&self) -> Result<PetState> {
        self.global
            .as_ref()
            .map(StateFile::load_or_default)
            .unwrap_or_else(|| Ok(PetState::default()))
    }

    pub fn record_commit(&self, scope: PetScope, record: ActivityRecord) -> Result<()> {
        if scope.includes_repo() {
            let repo = self
                .repo
                .as_ref()
                .ok_or(Error::MissingStateTarget("repo"))?;
            repo.record_commit(record.clone())?;
        }

        if scope.includes_global() {
            let global = self
                .global
                .as_ref()
                .ok_or(Error::MissingStateTarget("global"))?;
            global.record_commit(record)?;
        }

        Ok(())
    }
}

pub fn repo_state_path(git_dir: impl AsRef<Path>) -> PathBuf {
    git_dir.as_ref().join("commitchi").join("state.json")
}

pub fn global_state_path() -> Result<PathBuf> {
    if let Some(path) = env::var_os("COMMITCHI_DATA_DIR") {
        return Ok(PathBuf::from(path).join("state.json"));
    }

    if let Some(path) = env::var_os("XDG_DATA_HOME") {
        return Ok(PathBuf::from(path).join("commitchi").join("state.json"));
    }

    if cfg!(target_os = "windows") {
        return env::var_os("APPDATA")
            .map(|path| PathBuf::from(path).join("commitchi").join("state.json"))
            .ok_or(Error::MissingDataDirectory);
    }

    if cfg!(target_os = "macos") {
        return env::var_os("HOME")
            .map(|home| {
                PathBuf::from(home)
                    .join("Library")
                    .join("Application Support")
                    .join("commitchi")
                    .join("state.json")
            })
            .ok_or(Error::MissingDataDirectory);
    }

    env::var_os("HOME")
        .map(|home| {
            PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("commitchi")
                .join("state.json")
        })
        .ok_or(Error::MissingDataDirectory)
}

pub fn repo_id_from_path(path: impl AsRef<Path>) -> String {
    path.as_ref()
        .canonicalize()
        .unwrap_or_else(|_| path.as_ref().to_path_buf())
        .to_string_lossy()
        .into_owned()
}

pub fn now_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record_at(hours_ago: i64, now: i64) -> ActivityRecord {
        ActivityRecord::new(
            "repo",
            format!("hash-{hours_ago}"),
            now - hours_ago * 3_600,
            now - hours_ago * 3_600,
        )
    }

    #[test]
    fn mood_decays_across_thresholds() {
        let now = 10_000_000;
        let config = MoodConfig::default();
        let mut state = PetState::new();

        state.record_commit(record_at(12, now));
        assert_eq!(state.mood_at(now, config), Mood::Thriving);

        state.record_commit(record_at(36, now));
        assert_eq!(state.mood_at(now, config), Mood::Thriving);

        let mut stale = PetState::new();
        stale.record_commit(record_at(72, now));
        assert_eq!(stale.mood_at(now, config), Mood::Neutral);

        let mut old = PetState::new();
        old.record_commit(record_at(200, now));
        assert_eq!(old.mood_at(now, config), Mood::Sulking);
    }

    #[test]
    fn recent_consistency_softens_decay() {
        let now = 10_000_000;
        let config = MoodConfig::default();
        let mut state = PetState::new();

        state.record_commit(record_at(72, now));
        state.record_commit(record_at(80, now));
        state.record_commit(record_at(88, now));

        assert_eq!(state.mood_at(now, config), Mood::Content);
    }

    #[test]
    fn state_file_round_trips_json() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let file = StateFile::new(tmp.path().join("state.json"));
        let mut state = PetState::new();
        state.record_commit(ActivityRecord::new("repo", "abc123", 1, 2));

        file.save(&state).expect("save state");
        let loaded = file.load().expect("load state");

        assert_eq!(loaded, state);
        assert_eq!(loaded.version(), STATE_VERSION);
    }

    #[test]
    fn scope_recording_writes_selected_targets() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let repo_path = tmp.path().join("repo.json");
        let global_path = tmp.path().join("global.json");
        let files = PetStateFiles::from_paths(Some(repo_path), Some(global_path));
        let record = ActivityRecord::new("repo", "abc123", 1, 2);

        files
            .record_commit(PetScope::Both, record)
            .expect("record both");

        assert_eq!(
            files
                .load_repo_or_default()
                .expect("repo state")
                .records()
                .len(),
            1
        );
        assert_eq!(
            files
                .load_global_or_default()
                .expect("global state")
                .records()
                .len(),
            1
        );
    }

    #[test]
    fn repo_state_lives_under_git_metadata() {
        assert_eq!(
            repo_state_path("/repo/.git"),
            PathBuf::from("/repo/.git/commitchi/state.json")
        );
    }
}
