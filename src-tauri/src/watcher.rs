use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Duration;

use notify::{RecursiveMode, Watcher};
use tauri::{AppHandle, Runtime};

use crate::git_context;
use crate::notifier;
use crate::storage;

const REFRESH_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Debug)]
pub enum WatcherError {
    Init(String),
    Spawn(String),
}

impl std::fmt::Display for WatcherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WatcherError::Init(msg) => write!(f, "watcher init failed: {msg}"),
            WatcherError::Spawn(msg) => write!(f, "watcher thread spawn failed: {msg}"),
        }
    }
}

impl std::error::Error for WatcherError {}

pub fn start_watcher<R: Runtime>(app: AppHandle<R>) -> Result<(), WatcherError> {
    let (tx, rx) = std::sync::mpsc::channel();
    let watcher = notify::recommended_watcher(tx).map_err(|e| WatcherError::Init(e.to_string()))?;

    std::thread::Builder::new()
        .name("branch-watcher".to_string())
        .spawn(move || run(app, watcher, rx))
        .map_err(|e| WatcherError::Spawn(e.to_string()))?;

    Ok(())
}

fn run<R: Runtime>(
    app: AppHandle<R>,
    mut watcher: notify::RecommendedWatcher,
    rx: Receiver<notify::Result<notify::Event>>,
) {
    let mut watched: HashMap<PathBuf, String> = HashMap::new();

    loop {
        refresh(&mut watcher, &mut watched);

        match rx.recv_timeout(REFRESH_INTERVAL) {
            Ok(Ok(event)) => {
                for path in &event.paths {
                    if let Some(repo) = watched.get(path) {
                        handle_head_change(&app, repo);
                    }
                }
            }
            Ok(Err(_)) => {}
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
}

fn refresh(watcher: &mut notify::RecommendedWatcher, watched: &mut HashMap<PathBuf, String>) {
    let repos = match storage::distinct_repos() {
        Ok(repos) => repos,
        Err(_) => return,
    };

    let desired: HashMap<PathBuf, String> = repos
        .into_iter()
        .map(|repo| (head_path(&repo), repo))
        .collect();

    let removed: Vec<PathBuf> = watched
        .keys()
        .filter(|head| !desired.contains_key(*head))
        .cloned()
        .collect();
    for head in removed {
        let _ = watcher.unwatch(&head);
        watched.remove(&head);
    }

    for (head, repo) in &desired {
        if !watched.contains_key(head) && watcher.watch(head, RecursiveMode::NonRecursive).is_ok() {
            watched.insert(head.clone(), repo.clone());
        }
    }
}

fn handle_head_change<R: Runtime>(app: &AppHandle<R>, repo: &str) {
    let branch = match git_context::read_branch(repo) {
        Ok(branch) => branch,
        Err(_) => return,
    };

    let note = match storage::latest_note_for_branch(repo, &branch) {
        Ok(Some(note)) => note,
        _ => return,
    };

    let _ = notifier::notify_branch_note(app, &note);
}

fn head_path(repo: &str) -> PathBuf {
    Path::new(repo).join(".git").join("HEAD")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn head_path_appends_git_head() {
        let p = head_path("/home/dev/repo");
        assert_eq!(p, PathBuf::from("/home/dev/repo/.git/HEAD"));
    }

    #[test]
    fn head_path_ends_with_head_file() {
        let p = head_path("/home/dev/repo");
        assert_eq!(p.file_name().and_then(|n| n.to_str()), Some("HEAD"));
        assert_eq!(p.parent().and_then(|n| n.file_name()).and_then(|n| n.to_str()), Some(".git"));
    }

    #[test]
    fn watcher_error_display_contains_message() {
        let err = WatcherError::Init("boom".to_string());
        assert!(err.to_string().contains("boom"));
    }
}
