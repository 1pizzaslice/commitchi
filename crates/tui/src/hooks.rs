use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use commitchi_pet::PetScope;

const HOOK_BEGIN: &str = "# commitchi hook begin";
const HOOK_END: &str = "# commitchi hook end";

pub fn install_post_commit_hook(git_dir: impl AsRef<Path>, scope: PetScope) -> io::Result<PathBuf> {
    let hook_path = git_dir.as_ref().join("hooks").join("post-commit");
    if let Some(parent) = hook_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let existing = match fs::read_to_string(&hook_path) {
        Ok(contents) => Some(contents),
        Err(err) if err.kind() == io::ErrorKind::NotFound => None,
        Err(err) => return Err(err),
    };
    let contents = install_hook_contents(existing.as_deref(), &managed_block(scope));
    fs::write(&hook_path, contents)?;
    make_executable(&hook_path)?;

    Ok(hook_path)
}

fn managed_block(scope: PetScope) -> String {
    format!(
        "{HOOK_BEGIN}\nif command -v commitchi >/dev/null 2>&1; then\n  commitchi hook post-commit --scope {scope}\nfi\n{HOOK_END}\n"
    )
}

fn install_hook_contents(existing: Option<&str>, block: &str) -> String {
    match existing {
        None | Some("") => format!("#!/bin/sh\n\n{block}"),
        Some(existing) => upsert_managed_block(existing, block),
    }
}

fn upsert_managed_block(existing: &str, block: &str) -> String {
    let Some(start) = existing.find(HOOK_BEGIN) else {
        let mut contents = existing.to_owned();
        if !contents.ends_with('\n') {
            contents.push('\n');
        }
        contents.push('\n');
        contents.push_str(block);
        return contents;
    };

    let Some(end_offset) = existing[start..].find(HOOK_END) else {
        let mut contents = existing.to_owned();
        if !contents.ends_with('\n') {
            contents.push('\n');
        }
        contents.push('\n');
        contents.push_str(block);
        return contents;
    };

    let mut end = start + end_offset + HOOK_END.len();
    if existing[end..].starts_with('\n') {
        end += 1;
    }

    let mut contents = String::new();
    contents.push_str(&existing[..start]);
    contents.push_str(block);
    contents.push_str(&existing[end..]);
    contents
}

#[cfg(unix)]
fn make_executable(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(permissions.mode() | 0o111);
    fs::set_permissions(path, permissions)
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_new_hook_with_managed_block() {
        let contents = install_hook_contents(None, &managed_block(PetScope::Both));

        assert!(contents.starts_with("#!/bin/sh"));
        assert!(contents.contains("commitchi hook post-commit --scope both"));
    }

    #[test]
    fn appends_to_existing_hook() {
        let contents = install_hook_contents(
            Some("#!/bin/sh\necho existing\n"),
            &managed_block(PetScope::Repo),
        );

        assert!(contents.contains("echo existing"));
        assert!(contents.contains("commitchi hook post-commit --scope repo"));
    }

    #[test]
    fn replaces_existing_managed_block() {
        let existing = install_hook_contents(None, &managed_block(PetScope::Repo));
        let contents = install_hook_contents(Some(&existing), &managed_block(PetScope::Global));

        assert!(!contents.contains("--scope repo"));
        assert!(contents.contains("--scope global"));
        assert_eq!(contents.matches(HOOK_BEGIN).count(), 1);
    }
}
