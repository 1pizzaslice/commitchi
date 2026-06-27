use std::fs;
use std::path::Path;

use commitchi_core::{DiffLineKind, DiffOptions, FileStatus, RepoHandle};
use git2::{build::CheckoutBuilder, Oid, Repository, Signature};
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
        self.write_bytes(path, contents.as_bytes());
    }

    fn write_bytes(&self, path: &str, contents: &[u8]) {
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

    fn rename_file(&self, old_path: &str, new_path: &str) {
        fs::rename(self.path().join(old_path), self.path().join(new_path)).expect("rename file");
        let mut index = self.repo.index().expect("index");
        index
            .remove_path(Path::new(old_path))
            .expect("remove old path");
        index.add_path(Path::new(new_path)).expect("add new path");
    }

    fn commit(&self, message: &str) -> Oid {
        let parent = self
            .repo
            .head()
            .ok()
            .and_then(|head| head.target())
            .into_iter()
            .collect::<Vec<_>>();

        self.commit_with_parents(message, &parent)
    }

    fn commit_with_parents(&self, message: &str, parents: &[Oid]) -> Oid {
        let signature = Signature::now("Test User", "test@example.com").expect("signature");
        let mut index = self.repo.index().expect("index");
        index.write().expect("write index");
        let tree_id = index.write_tree().expect("write tree");
        let tree = self.repo.find_tree(tree_id).expect("tree");
        let parent_commits = parents
            .iter()
            .map(|oid| self.repo.find_commit(*oid).expect("parent commit"))
            .collect::<Vec<_>>();
        let parent_refs = parent_commits.iter().collect::<Vec<_>>();

        self.repo
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                message,
                &tree,
                &parent_refs,
            )
            .expect("commit")
    }

    fn checkout_branch(&self, branch: &str) {
        self.repo
            .set_head(&format!("refs/heads/{branch}"))
            .expect("set branch head");
        self.repo
            .checkout_head(Some(CheckoutBuilder::new().force()))
            .expect("checkout branch");
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
fn renamed_file_uses_renamed_status() {
    let fixture = Fixture::new();
    fixture.write_file("old.txt", "same contents\n");
    fixture.commit("add old");
    fixture.rename_file("old.txt", "new.txt");
    let rename = fixture.commit("rename old");

    let handle = RepoHandle::discover(fixture.path()).expect("discover repo");
    let diff = handle
        .diff_for_commit(rename.to_string(), DiffOptions::default())
        .expect("diff");

    assert_eq!(diff.files[0].status, FileStatus::Renamed);
    assert_eq!(diff.files[0].old_path.as_deref(), Some("old.txt"));
    assert_eq!(diff.files[0].new_path.as_deref(), Some("new.txt"));
}

#[test]
fn binary_file_reports_binary_line() {
    let fixture = Fixture::new();
    fixture.write_bytes("image.bin", b"\0\0\0commitchi\0");
    let binary = fixture.commit("add binary");

    let handle = RepoHandle::discover(fixture.path()).expect("discover repo");
    let diff = handle
        .diff_for_commit(binary.to_string(), DiffOptions::default())
        .expect("diff");

    assert!(diff.files[0].binary);
    assert!(diff.files[0]
        .lines
        .iter()
        .any(|line| line.kind == DiffLineKind::Binary));
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
fn large_diff_truncates_file_list() {
    let fixture = Fixture::new();
    fixture.write_file("seed.txt", "seed\n");
    fixture.commit("seed");
    fixture.write_file("a.txt", "a\n");
    fixture.write_file("b.txt", "b\n");
    fixture.write_file("c.txt", "c\n");
    let many_files = fixture.commit("many files");

    let handle = RepoHandle::discover(fixture.path()).expect("discover repo");
    let diff = handle
        .diff_for_commit(
            many_files.to_string(),
            DiffOptions {
                line_limit: 2_000,
                file_limit: 1,
                context_lines: 3,
            },
        )
        .expect("diff");

    assert!(diff.truncated);
    assert!(diff
        .files
        .iter()
        .any(|file| file.status == FileStatus::Truncated));
}

#[test]
fn merge_commit_diff_uses_first_parent() {
    let fixture = Fixture::new();
    fixture.write_file("base.txt", "base\n");
    let base = fixture.commit("base");
    let main_branch = fixture
        .repo
        .head()
        .expect("head")
        .shorthand()
        .expect("branch name")
        .to_owned();

    let base_commit = fixture.repo.find_commit(base).expect("base commit");
    fixture
        .repo
        .branch("side", &base_commit, false)
        .expect("create side branch");

    fixture.checkout_branch("side");
    fixture.write_file("side.txt", "side\n");
    let side = fixture.commit("side");

    fixture.checkout_branch(&main_branch);
    fixture.write_file("main.txt", "main\n");
    let main = fixture.commit("main");

    fixture.write_file("side.txt", "side\n");
    let merge = fixture.commit_with_parents("merge side", &[main, side]);

    let handle = RepoHandle::discover(fixture.path()).expect("discover repo");
    let diff = handle
        .diff_for_commit(merge.to_string(), DiffOptions::default())
        .expect("diff");
    let paths = diff
        .files
        .iter()
        .map(|file| file.display_path())
        .collect::<Vec<_>>();

    assert!(paths.contains(&"side.txt".to_owned()));
    assert!(!paths.contains(&"main.txt".to_owned()));
}

#[test]
fn empty_repo_returns_no_summaries() {
    let fixture = Fixture::new();
    let handle = RepoHandle::discover(fixture.path()).expect("discover repo");

    assert!(handle.commit_summaries().expect("summaries").is_empty());
}
