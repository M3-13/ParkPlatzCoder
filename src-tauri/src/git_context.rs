//! Git context capture via libgit2.
//!
//! Everything in this module that touches git goes through the `git2`
//! (libgit2) crate — no shell calls to `git` are ever made.

use crate::model::GitContext;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GitError {
    #[error("failed to open git repository: {0}")]
    Open(#[from] git2::Error),
    #[error("repository has no HEAD commit")]
    NoHead,
    #[error("repository has no working directory")]
    NoWorkdir,
}

/// The last repository `detect_repo` positively identified. Used as the
/// fallback when there is no active window.
static LAST_KNOWN_REPO: OnceLock<Mutex<Option<String>>> = OnceLock::new();

fn last_known_repo_slot() -> &'static Mutex<Option<String>> {
    LAST_KNOWN_REPO.get_or_init(|| Mutex::new(None))
}

fn remember_repo(path: &str) {
    if let Ok(mut slot) = last_known_repo_slot().lock() {
        *slot = Some(path.to_string());
    }
}

fn last_known_repo() -> Option<String> {
    match last_known_repo_slot().lock() {
        Ok(guard) => guard.clone(),
        Err(poisoned) => poisoned.into_inner().clone(),
    }
}

/// Returns the absolute path of the enclosing repository, if any.
///
/// On Linux this asks the X11 window manager for the active window
/// (`_NET_ACTIVE_WINDOW`), resolves it to a process id (`_NET_WM_PID`), reads
/// `/proc/<pid>/cwd` and walks up towards the nearest `.git`. Without an
/// active window it falls back to the last repository it positively detected;
/// with an active window that sits outside any repository it returns
/// `Ok(None)`.
pub fn detect_repo() -> Result<Option<String>, GitError> {
    match active_window_repo() {
        WindowOutcome::Found(repo) => {
            remember_repo(&repo);
            Ok(Some(repo))
        }
        WindowOutcome::NoRepo => Ok(None),
        WindowOutcome::NoWindow => Ok(last_known_repo()),
    }
}

// On non-Linux there is no active-window detection, so only `NoWindow` is
// constructed; the other variants are part of the Linux X11 path.
#[allow(dead_code)]
enum WindowOutcome {
    Found(String),
    NoRepo,
    NoWindow,
}

#[cfg(target_os = "linux")]
fn active_window_repo() -> WindowOutcome {
    let pid = match active_window_pid() {
        Some(p) => p,
        None => return WindowOutcome::NoWindow,
    };
    let cwd = match proc_cwd(pid) {
        Some(c) => c,
        None => return WindowOutcome::NoRepo,
    };
    match find_repo_root(&cwd) {
        Some(repo) => WindowOutcome::Found(repo.to_string_lossy().into_owned()),
        None => WindowOutcome::NoRepo,
    }
}

#[cfg(not(target_os = "linux"))]
fn active_window_repo() -> WindowOutcome {
    WindowOutcome::NoWindow
}

/// Read the full git context of a repository: repo path, branch, last commit
/// hash and changed files (git status).
pub fn read_git_context(repo_path: &str) -> Result<GitContext, GitError> {
    let repo = git2::Repository::discover(repo_path)?;
    let root = repo
        .workdir()
        .map(Path::to_path_buf)
        .or_else(|| repo.path().parent().map(Path::to_path_buf))
        .ok_or(GitError::NoWorkdir)?;
    Ok(GitContext {
        repo_path: normalize_path(&root),
        branch: branch_name(&repo)?,
        commit_hash: head_commit_hash(&repo)?,
        changed_files: changed_files(&repo)?,
    })
}

/// Read the current branch via libgit2.
pub fn read_branch(repo_path: &str) -> Result<String, GitError> {
    let repo = git2::Repository::discover(repo_path)?;
    branch_name(&repo)
}

fn branch_name(repo: &git2::Repository) -> Result<String, GitError> {
    let head = match repo.head() {
        Ok(h) => h,
        Err(e) if e.code() == git2::ErrorCode::UnbornBranch => {
            return unborn_branch_name(repo);
        }
        Err(e) => return Err(GitError::Open(e)),
    };
    match head.shorthand() {
        Ok(name) => Ok(name.to_string()),
        Err(_) => Ok(head.target().map(|oid| oid.to_string()).unwrap_or_default()),
    }
}

fn unborn_branch_name(repo: &git2::Repository) -> Result<String, GitError> {
    let head_ref = repo.find_reference("HEAD").map_err(GitError::Open)?;
    let name = head_ref
        .symbolic_target()
        .map_err(GitError::Open)?
        .and_then(|t| t.strip_prefix("refs/heads/"))
        .unwrap_or("HEAD");
    Ok(name.to_string())
}

fn head_commit_hash(repo: &git2::Repository) -> Result<String, GitError> {
    let head = repo.head().map_err(|e| {
        if e.code() == git2::ErrorCode::UnbornBranch {
            GitError::NoHead
        } else {
            GitError::Open(e)
        }
    })?;
    let commit = head.peel_to_commit().map_err(GitError::Open)?;
    Ok(commit.id().to_string())
}

fn changed_files(repo: &git2::Repository) -> Result<Vec<String>, GitError> {
    let mut opts = git2::StatusOptions::new();
    opts.include_untracked(true);
    let statuses = repo.statuses(Some(&mut opts)).map_err(GitError::Open)?;
    let mut changed: Vec<String> = Vec::new();
    for entry in statuses.iter() {
        let status = entry.status();
        if status.is_empty() || status.contains(git2::Status::IGNORED) {
            continue;
        }
        changed.push(entry.path().map_err(GitError::Open)?.to_string());
    }
    changed.sort();
    changed.dedup();
    Ok(changed)
}

fn normalize_path(path: &Path) -> String {
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let mut s = canonical.to_string_lossy().into_owned();
    #[cfg(windows)]
    {
        if let Some(stripped) = s.strip_prefix(r"\\?\") {
            s = stripped.to_string();
        }
    }
    s
}

/// Walk up from `start` towards the filesystem root looking for a `.git`.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn find_repo_root(start: &Path) -> Option<PathBuf> {
    let mut dir = if start.is_dir() {
        start.to_path_buf()
    } else {
        start.parent()?.to_path_buf()
    };
    loop {
        if dir.join(".git").exists() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

#[cfg(target_os = "linux")]
fn proc_cwd(pid: u32) -> Option<PathBuf> {
    std::fs::read_link(format!("/proc/{}/cwd", pid)).ok()
}

#[cfg(target_os = "linux")]
fn active_window_pid() -> Option<u32> {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as _};

    let (conn, screen_num) = x11rb::connect(None).ok()?;
    let root = conn.setup().roots[screen_num].root;

    let active_atom = conn
        .intern_atom(false, b"_NET_ACTIVE_WINDOW")
        .ok()?
        .reply()
        .ok()?
        .atom;

    let active_prop = conn
        .get_property(false, root, active_atom, AtomEnum::WINDOW, 0, 1)
        .ok()?
        .reply()
        .ok()?;

    let window = *active_prop.value32()?.first()?;
    if window == 0 {
        return None;
    }

    let pid_atom = conn
        .intern_atom(false, b"_NET_WM_PID")
        .ok()?
        .reply()
        .ok()?
        .atom;

    let pid_prop = conn
        .get_property(false, window, pid_atom, AtomEnum::CARDINAL, 0, 1)
        .ok()?
        .reply()
        .ok()?;

    pid_prop.value32()?.first().copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let n = TEMP_COUNTER.fetch_add(1, Ordering::SeqCst);
            let dir = std::env::temp_dir().join(format!(
                "parkplatzcoder_gitctx_{}_{}",
                std::process::id(),
                n
            ));
            std::fs::create_dir_all(&dir).unwrap();
            TempDir(dir)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn init_repo(dir: &Path) -> git2::Repository {
        let mut opts = git2::RepositoryInitOptions::new();
        opts.initial_head("main");
        git2::Repository::init_opts(dir, &opts).expect("init repo")
    }

    fn write_file(dir: &Path, name: &str, content: &str) {
        std::fs::write(dir.join(name), content).expect("write file");
    }

    fn commit_files(repo: &git2::Repository, files: &[&str], msg: &str) -> git2::Oid {
        let mut index = repo.index().expect("index");
        for f in files {
            index.add_path(Path::new(f)).expect("add path");
        }
        index.write().expect("index write");
        let tree_id = index.write_tree().expect("write tree");
        let tree = repo.find_tree(tree_id).expect("find tree");
        let sig = git2::Signature::now("Test", "test@example.com").expect("sig");
        match repo.head().ok().and_then(|h| h.target()) {
            Some(parent_id) => {
                let parent = repo.find_commit(parent_id).expect("find parent");
                repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &[&parent])
                    .expect("commit")
            }
            None => repo
                .commit(Some("HEAD"), &sig, &sig, msg, &tree, &[])
                .expect("commit"),
        }
    }

    #[test]
    fn read_branch_returns_the_current_branch() {
        let dir = TempDir::new();
        let repo = init_repo(dir.path());
        write_file(dir.path(), "hello.txt", "hello\n");
        commit_files(&repo, &["hello.txt"], "initial commit");

        let branch = read_branch(&dir.path().to_string_lossy()).unwrap();
        assert_eq!(branch, "main");
    }

    #[test]
    fn read_git_context_reports_branch_commit_and_changed_files() {
        let dir = TempDir::new();
        let repo = init_repo(dir.path());
        write_file(dir.path(), "hello.txt", "hello\n");
        write_file(dir.path(), "other.txt", "other\n");
        commit_files(&repo, &["hello.txt", "other.txt"], "initial commit");

        // Modify only one tracked file so the changed set is exact.
        write_file(dir.path(), "hello.txt", "changed\n");

        let ctx = read_git_context(&dir.path().to_string_lossy()).unwrap();

        assert_eq!(ctx.branch, "main");
        let expected_hash = repo
            .head()
            .unwrap()
            .peel_to_commit()
            .unwrap()
            .id()
            .to_string();
        assert_eq!(ctx.commit_hash, expected_hash);
        assert_eq!(ctx.changed_files, vec!["hello.txt".to_string()]);
        assert_eq!(ctx.repo_path, normalize_path(dir.path()));
    }

    #[test]
    fn read_git_context_works_from_a_nested_directory() {
        let dir = TempDir::new();
        let repo = init_repo(dir.path());
        write_file(dir.path(), "hello.txt", "hello\n");
        commit_files(&repo, &["hello.txt"], "initial commit");

        let nested = dir.path().join("sub").join("deep");
        std::fs::create_dir_all(&nested).unwrap();

        let ctx = read_git_context(&nested.to_string_lossy()).unwrap();
        assert_eq!(ctx.branch, "main");
        assert!(ctx.changed_files.is_empty());
        assert_eq!(ctx.repo_path, normalize_path(dir.path()));
    }

    #[test]
    fn detect_repo_falls_back_to_last_known_repo_without_window() {
        // No repository remembered yet -> None.
        *last_known_repo_slot().lock().unwrap() = None;
        assert_eq!(detect_repo().unwrap(), None);

        // Remember a repository -> the fallback returns it. This environment
        // (Windows dev machine, headless CI container) has no active X11
        // window, so `detect_repo` always takes the fallback path.
        let dir = TempDir::new();
        let repo = init_repo(dir.path());
        write_file(dir.path(), "hello.txt", "hello\n");
        commit_files(&repo, &["hello.txt"], "initial commit");

        let path = dir.path().to_string_lossy().into_owned();
        remember_repo(&path);
        assert_eq!(detect_repo().unwrap().as_deref(), Some(path.as_str()));
    }

    #[test]
    fn find_repo_root_walks_up_to_git_dir() {
        let dir = TempDir::new();
        let repo = init_repo(dir.path());
        let nested = dir.path().join("a").join("b").join("c");
        std::fs::create_dir_all(&nested).unwrap();

        let found = find_repo_root(&nested).expect("repo root found");
        assert_eq!(found.as_path(), dir.path());
        drop(repo);
    }
}
