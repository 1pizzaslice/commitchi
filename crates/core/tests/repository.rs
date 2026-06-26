use std::fs;
use std::path::Path;

use commitchi_core::{DiffLineKind, DiffOptions, FileStatus, RepoHandle};
use git2::{Oid, Repository, Signature};
use tempfile::TempDir;

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
        let full_path = self.path().join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).expect("create parent dir");
        }
        fs::write(full_path, contents).expect("write file");
        self.repo
            .index()
            .expect("index")
            .add_path(Path::new(path))
            .expect("add path");
    }

    fn remove_file(&self, path: &str) {
        fs::remove_file(self.path().join(path)).expect("remove file");
        self.repo
            .index()
            .expect("index")
            .remove_path(Path::new(path))
            .expect("remove path");
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

#[test]
fn discovers_repo_from_nested_path() {
    let fixture = Fixture::new();
    fixture.write_file("src/main.rs", "fn main() {}\n");
    fixture.commit("initial");

    let nested = fixture.path().join("src");
    let handle = RepoHandle::discover(&nested).expect("discover repo");

    assert_eq!(
        handle.root().canonicalize().expect("handle root"),
        fixture.path().canonicalize().expect("fixture root")
    );
}

#[test]
fn commit_summaries_are_oldest_to_newest() {
    let fixture = Fixture::new();
    fixture.write_file("story.txt", "one\n");
    fixture.commit("first");
    fixture.write_file("story.txt", "two\n");
    fixture.commit("second");
    fixture.write_file("story.txt", "three\n");
    fixture.commit("third");

    let handle = RepoHandle::discover(fixture.path()).expect("discover repo");
    let summaries = handle.commit_summaries().expect("summaries");
    let messages = summaries
        .iter()
        .map(|summary| summary.summary.as_str())
        .collect::<Vec<_>>();

    assert_eq!(messages, ["first", "second", "third"]);
}

#[test]
fn diff_for_commit_reports_structured_lines_and_stats() {
    let fixture = Fixture::new();
    fixture.write_file("story.txt", "one\nshared\n");
    fixture.commit("first");
    fixture.write_file("story.txt", "two\nshared\n");
    let second = fixture.commit("second");

    let handle = RepoHandle::discover(fixture.path()).expect("discover repo");
    let diff = handle
        .diff_for_commit(second.to_string(), DiffOptions::default())
        .expect("diff");

    assert_eq!(diff.stats.files_changed, 1);
    assert_eq!(diff.stats.additions, 1);
    assert_eq!(diff.stats.deletions, 1);
    assert_eq!(diff.files[0].status, FileStatus::Modified);
    assert!(diff.files[0]
        .lines
        .iter()
        .any(|line| line.kind == DiffLineKind::Addition && line.content == "two"));
    assert!(diff.files[0]
        .lines
        .iter()
        .any(|line| line.kind == DiffLineKind::Deletion && line.content == "one"));
}

#[test]
fn deleted_file_uses_deleted_status() {
    let fixture = Fixture::new();
    fixture.write_file("old.txt", "old\n");
    fixture.commit("add old");
    fixture.remove_file("old.txt");
    let delete = fixture.commit("delete old");

    let handle = RepoHandle::discover(fixture.path()).expect("discover repo");
    let diff = handle
        .diff_for_commit(delete.to_string(), DiffOptions::default())
        .expect("diff");

    assert_eq!(diff.files[0].status, FileStatus::Deleted);
    assert_eq!(diff.stats.deletions, 1);
}

#[test]
fn large_diff_truncates_lines() {
    let fixture = Fixture::new();
    fixture.write_file("large.txt", "seed\n");
    fixture.commit("seed");
    let contents = (0..100)
        .map(|index| format!("line {index}\n"))
        .collect::<String>();
    fixture.write_file("large.txt", &contents);
    let large = fixture.commit("large");

    let handle = RepoHandle::discover(fixture.path()).expect("discover repo");
    let diff = handle
        .diff_for_commit(
            large.to_string(),
            DiffOptions {
                line_limit: 2,
                file_limit: 100,
                context_lines: 3,
            },
        )
        .expect("diff");

    assert!(diff.truncated);
    assert!(diff
        .files
        .iter()
        .flat_map(|file| &file.lines)
        .any(|line| line.kind == DiffLineKind::Truncated));
}

#[test]
fn empty_repo_returns_no_summaries() {
    let fixture = Fixture::new();
    let handle = RepoHandle::discover(fixture.path()).expect("discover repo");

    assert!(handle.commit_summaries().expect("summaries").is_empty());
}
