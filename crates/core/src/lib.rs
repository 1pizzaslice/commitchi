use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};

use git2::{
    Delta, DiffBinary, DiffDelta, DiffFindOptions, DiffHunk, DiffLine as GitDiffLine,
    DiffOptions as GitDiffOptions, Repository, Sort,
};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to open git repository at {path}: {source}")]
    OpenRepository { path: PathBuf, source: git2::Error },

    #[error("repository has no commits")]
    EmptyRepository,

    #[error("commit not found: {0}")]
    CommitNotFound(String),

    #[error(transparent)]
    Git(#[from] git2::Error),
}

pub struct RepoHandle {
    repo: Repository,
    root: PathBuf,
}

impl std::fmt::Debug for RepoHandle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RepoHandle")
            .field("root", &self.root)
            .finish_non_exhaustive()
    }
}

impl RepoHandle {
    pub fn discover(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let repo = Repository::discover(path).map_err(|source| Error::OpenRepository {
            path: path.to_path_buf(),
            source,
        })?;
        let root = repo
            .workdir()
            .map(Path::to_path_buf)
            .or_else(|| repo.path().parent().map(Path::to_path_buf))
            .unwrap_or_else(|| repo.path().to_path_buf());

        Ok(Self { repo, root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn git_dir(&self) -> &Path {
        self.repo.path()
    }

    pub fn head_commit_summary(&self) -> Result<CommitSummary> {
        let head = self.repo.head().map_err(|err| {
            if is_empty_head(&err) {
                Error::EmptyRepository
            } else {
                Error::Git(err)
            }
        })?;
        let commit = head.peel_to_commit()?;

        Ok(CommitSummary::from_commit(&commit))
    }

    pub fn commit_summaries(&self) -> Result<Vec<CommitSummary>> {
        self.commit_page(0, usize::MAX).map(|page| page.commits)
    }

    pub fn commit_page(&self, offset: usize, limit: usize) -> Result<CommitPage> {
        if limit == 0 {
            return Ok(CommitPage {
                commits: Vec::new(),
                offset,
                has_more: true,
            });
        }

        let mut revwalk = self.repo.revwalk()?;
        match revwalk.push_head() {
            Ok(()) => {}
            Err(err) if is_empty_head(&err) => {
                return Ok(CommitPage {
                    commits: Vec::new(),
                    offset,
                    has_more: false,
                });
            }
            Err(err) => return Err(Error::Git(err)),
        }
        revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME | Sort::REVERSE)?;

        let take_limit = limit.saturating_add(1);
        let mut commits = Vec::new();

        for oid in revwalk.skip(offset).take(take_limit) {
            let oid = oid?;
            if commits.len() == limit {
                return Ok(CommitPage {
                    commits,
                    offset,
                    has_more: true,
                });
            }

            let commit = self.repo.find_commit(oid)?;
            commits.push(CommitSummary::from_commit(&commit));
        }

        Ok(CommitPage {
            commits,
            offset,
            has_more: false,
        })
    }

    pub fn diff_for_commit(
        &self,
        commit_hash: impl AsRef<str>,
        options: DiffOptions,
    ) -> Result<StructuredDiff> {
        let commit_hash = commit_hash.as_ref();
        let object = self
            .repo
            .revparse_single(commit_hash)
            .map_err(|_| Error::CommitNotFound(commit_hash.to_owned()))?;
        let commit = object
            .peel_to_commit()
            .map_err(|_| Error::CommitNotFound(commit_hash.to_owned()))?;

        let new_tree = commit.tree()?;
        let old_tree = if commit.parent_count() == 0 {
            None
        } else {
            Some(commit.parent(0)?.tree()?)
        };

        let mut diff_options = GitDiffOptions::new();
        diff_options.context_lines(options.context_lines);

        let mut diff = self.repo.diff_tree_to_tree(
            old_tree.as_ref(),
            Some(&new_tree),
            Some(&mut diff_options),
        )?;

        let mut find_options = DiffFindOptions::new();
        find_options.renames(true);
        diff.find_similar(Some(&mut find_options))?;

        let mut files = collect_files(&diff, options)?;
        let raw_stats = diff.stats()?;
        let file_limit_hit = files.len() < raw_stats.files_changed();
        let line_limit_hit = files.iter().any(|file| file.truncated);
        let truncated = file_limit_hit || line_limit_hit;

        if file_limit_hit {
            files.push(FileDiff {
                old_path: None,
                new_path: None,
                status: FileStatus::Truncated,
                additions: 0,
                deletions: 0,
                binary: false,
                truncated: true,
                lines: vec![DiffLine {
                    kind: DiffLineKind::Truncated,
                    old_lineno: None,
                    new_lineno: None,
                    content: format!(
                        "file list truncated after {} files; {} files changed",
                        options.file_limit,
                        raw_stats.files_changed()
                    ),
                }],
            });
        }

        Ok(StructuredDiff {
            commit_hash: commit.id().to_string(),
            files,
            stats: DiffStats {
                files_changed: raw_stats.files_changed(),
                additions: raw_stats.insertions(),
                deletions: raw_stats.deletions(),
            },
            truncated,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitSummary {
    pub hash: String,
    pub short_hash: String,
    pub summary: String,
    pub author_name: String,
    pub author_email: String,
    pub time_seconds: i64,
    pub parent_count: usize,
}

impl CommitSummary {
    fn from_commit(commit: &git2::Commit<'_>) -> Self {
        let hash = commit.id().to_string();
        let short_hash = hash.chars().take(8).collect();
        let author = commit.author();

        Self {
            hash,
            short_hash,
            summary: commit.summary().unwrap_or("(no summary)").to_owned(),
            author_name: author.name().unwrap_or("unknown").to_owned(),
            author_email: author.email().unwrap_or("").to_owned(),
            time_seconds: commit.time().seconds(),
            parent_count: commit.parent_count(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitPage {
    pub commits: Vec<CommitSummary>,
    pub offset: usize,
    pub has_more: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiffOptions {
    pub line_limit: usize,
    pub file_limit: usize,
    pub context_lines: u32,
}

impl Default for DiffOptions {
    fn default() -> Self {
        Self {
            line_limit: 2_000,
            file_limit: 100,
            context_lines: 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuredDiff {
    pub commit_hash: String,
    pub files: Vec<FileDiff>,
    pub stats: DiffStats,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDiff {
    pub old_path: Option<String>,
    pub new_path: Option<String>,
    pub status: FileStatus,
    pub additions: usize,
    pub deletions: usize,
    pub binary: bool,
    pub truncated: bool,
    pub lines: Vec<DiffLine>,
}

impl FileDiff {
    pub fn display_path(&self) -> String {
        self.new_path
            .as_ref()
            .or(self.old_path.as_ref())
            .cloned()
            .unwrap_or_else(|| "(unknown path)".to_owned())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    Added,
    Deleted,
    Modified,
    Renamed,
    Copied,
    TypeChanged,
    Conflicted,
    Truncated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub old_lineno: Option<u32>,
    pub new_lineno: Option<u32>,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineKind {
    Context,
    Addition,
    Deletion,
    HunkHeader,
    FileHeader,
    Binary,
    Truncated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffStats {
    pub files_changed: usize,
    pub additions: usize,
    pub deletions: usize,
}

fn collect_files(diff: &git2::Diff<'_>, options: DiffOptions) -> Result<Vec<FileDiff>> {
    let files = RefCell::new(Vec::<FileDiff>::new());
    let line_count = Cell::new(0usize);
    let accepting_file = Cell::new(true);
    let line_limit_hit = Cell::new(false);

    let mut file_cb = |delta: DiffDelta<'_>, _progress: f32| {
        let current_count = files.borrow().len();
        if current_count >= options.file_limit {
            accepting_file.set(false);
            return true;
        }

        accepting_file.set(true);
        files.borrow_mut().push(file_from_delta(&delta));

        if !line_limit_hit.get() {
            let line = DiffLine {
                kind: DiffLineKind::FileHeader,
                old_lineno: None,
                new_lineno: None,
                content: file_header(&delta),
            };
            push_line(
                &files,
                &line_count,
                &line_limit_hit,
                options.line_limit,
                line,
            );
        }

        true
    };

    let mut binary_cb = |_: DiffDelta<'_>, _binary: DiffBinary<'_>| {
        if !accepting_file.get() {
            return true;
        }

        if let Some(file) = files.borrow_mut().last_mut() {
            file.binary = true;
        }

        if !line_limit_hit.get() {
            let line = DiffLine {
                kind: DiffLineKind::Binary,
                old_lineno: None,
                new_lineno: None,
                content: "binary file changed".to_owned(),
            };
            push_line(
                &files,
                &line_count,
                &line_limit_hit,
                options.line_limit,
                line,
            );
        }

        true
    };

    let mut hunk_cb = |_: DiffDelta<'_>, hunk: DiffHunk<'_>| {
        if !accepting_file.get() || line_limit_hit.get() {
            return true;
        }

        let line = DiffLine {
            kind: DiffLineKind::HunkHeader,
            old_lineno: None,
            new_lineno: None,
            content: hunk_header(hunk),
        };
        push_line(
            &files,
            &line_count,
            &line_limit_hit,
            options.line_limit,
            line,
        );
        true
    };

    let mut line_cb = |_: DiffDelta<'_>, _hunk: Option<DiffHunk<'_>>, line: GitDiffLine<'_>| {
        if !accepting_file.get() || line_limit_hit.get() {
            return true;
        }

        let diff_line = DiffLine {
            kind: line_kind(line.origin()),
            old_lineno: line.old_lineno(),
            new_lineno: line.new_lineno(),
            content: line_content(line),
        };
        push_line(
            &files,
            &line_count,
            &line_limit_hit,
            options.line_limit,
            diff_line,
        );
        true
    };

    diff.foreach(
        &mut file_cb,
        Some(&mut binary_cb),
        Some(&mut hunk_cb),
        Some(&mut line_cb),
    )?;

    Ok(files.into_inner())
}

fn push_line(
    files: &RefCell<Vec<FileDiff>>,
    line_count: &Cell<usize>,
    line_limit_hit: &Cell<bool>,
    line_limit: usize,
    line: DiffLine,
) {
    if line_count.get() >= line_limit {
        mark_current_file_truncated(files);
        line_limit_hit.set(true);
        return;
    }

    if let Some(file) = files.borrow_mut().last_mut() {
        match line.kind {
            DiffLineKind::Addition => file.additions += 1,
            DiffLineKind::Deletion => file.deletions += 1,
            _ => {}
        }
        file.lines.push(line);
        line_count.set(line_count.get() + 1);
    }
}

fn mark_current_file_truncated(files: &RefCell<Vec<FileDiff>>) {
    if let Some(file) = files.borrow_mut().last_mut() {
        file.truncated = true;
        if !matches!(
            file.lines.last().map(|line| line.kind),
            Some(DiffLineKind::Truncated)
        ) {
            file.lines.push(DiffLine {
                kind: DiffLineKind::Truncated,
                old_lineno: None,
                new_lineno: None,
                content: "diff truncated at configured line limit".to_owned(),
            });
        }
    }
}

fn file_from_delta(delta: &DiffDelta<'_>) -> FileDiff {
    FileDiff {
        old_path: path_string(delta.old_file().path()),
        new_path: path_string(delta.new_file().path()),
        status: status_from_delta(delta.status()),
        additions: 0,
        deletions: 0,
        binary: false,
        truncated: false,
        lines: Vec::new(),
    }
}

fn file_header(delta: &DiffDelta<'_>) -> String {
    let old_path = path_string(delta.old_file().path()).unwrap_or_else(|| "/dev/null".to_owned());
    let new_path = path_string(delta.new_file().path()).unwrap_or_else(|| "/dev/null".to_owned());

    if old_path == new_path {
        format!("diff -- {}", new_path)
    } else {
        format!("diff -- {} -> {}", old_path, new_path)
    }
}

fn hunk_header(hunk: DiffHunk<'_>) -> String {
    let header = String::from_utf8_lossy(hunk.header()).to_string();
    trim_line_end(header)
}

fn line_content(line: GitDiffLine<'_>) -> String {
    trim_line_end(String::from_utf8_lossy(line.content()).to_string())
}

fn trim_line_end(mut value: String) -> String {
    while value.ends_with('\n') || value.ends_with('\r') {
        value.pop();
    }
    value
}

fn line_kind(origin: char) -> DiffLineKind {
    match origin {
        '+' => DiffLineKind::Addition,
        '-' => DiffLineKind::Deletion,
        _ => DiffLineKind::Context,
    }
}

fn status_from_delta(delta: Delta) -> FileStatus {
    match delta {
        Delta::Added => FileStatus::Added,
        Delta::Deleted => FileStatus::Deleted,
        Delta::Renamed => FileStatus::Renamed,
        Delta::Copied => FileStatus::Copied,
        Delta::Typechange => FileStatus::TypeChanged,
        Delta::Conflicted => FileStatus::Conflicted,
        _ => FileStatus::Modified,
    }
}

fn path_string(path: Option<&Path>) -> Option<String> {
    path.map(|path| path.to_string_lossy().into_owned())
}

fn is_empty_head(err: &git2::Error) -> bool {
    matches!(
        err.code(),
        git2::ErrorCode::UnbornBranch | git2::ErrorCode::NotFound
    ) || err.message().contains("not found")
        || err.message().contains("unborn")
}
