use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};

use notify::{RecommendedWatcher, RecursiveMode, Watcher};

#[derive(Debug)]
pub struct StateWatcher {
    _watcher: RecommendedWatcher,
    receiver: Receiver<notify::Result<notify::Event>>,
    watched_files: Vec<PathBuf>,
}

impl StateWatcher {
    pub fn watch(watched_files: Vec<PathBuf>) -> notify::Result<Option<Self>> {
        if watched_files.is_empty() {
            return Ok(None);
        }

        let (sender, receiver) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(move |event| {
            let _ = sender.send(event);
        })?;

        let mut watched_dirs = BTreeSet::new();
        for path in &watched_files {
            if let Some(parent) = path.parent() {
                watched_dirs.insert(parent.to_path_buf());
            }
        }

        for dir in watched_dirs {
            watcher.watch(&dir, RecursiveMode::NonRecursive)?;
        }

        Ok(Some(Self {
            _watcher: watcher,
            receiver,
            watched_files,
        }))
    }

    pub fn drain_changed(&mut self) -> bool {
        let mut changed = false;

        while let Ok(event) = self.receiver.try_recv() {
            match event {
                Ok(event) => {
                    if event.paths.iter().any(|path| {
                        self.watched_files
                            .iter()
                            .any(|watched| same_path(path, watched))
                    }) {
                        changed = true;
                    }
                }
                Err(_) => {
                    changed = true;
                }
            }
        }

        changed
    }
}

fn same_path(left: &Path, right: &Path) -> bool {
    left == right
        || left.file_name() == right.file_name()
            && left.parent().and_then(Path::file_name) == right.parent().and_then(Path::file_name)
}
