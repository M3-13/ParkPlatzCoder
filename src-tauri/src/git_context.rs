use std::path::Path;
use std::sync::{Mutex, OnceLock};

use crate::model::GitContext;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GitError {
    #[error("git operation failed: {0}")]
    Git(#[from] git2::Error),
    #[error("repository is bare and has no working directory")]
    BareRepository,
    #[error("failed to read filesystem path: {0}")]
    Io(#[from] std::io::Error),
    #[error("window manager lookup failed: {0}")]
    WindowManager(String),
}

/// Returns the absolute path of the enclosing repository, if any.
///
/// Detection order:
/// 1. The repository of the currently active window (X11 `_NET_ACTIVE_WINDOW`
///    -> `_NET_WM_PID` -> `/proc/<pid>/cwd` -> nearest `.git` ancestor).
/// 2. If no active window can be resolved (or the platform has no X11), the
///    last repository that was successfully detected this process.
/// 3. Otherwise `None`.
pub fn detect_repo() -> Result<Option<String>, GitError> {
    if let Some(path) = detect_from_active_window()? {
        remember_repo(&path);
        return Ok(Some(path));
    }
    Ok(read_remembered_repo())
}

fn last_known_repo() -> &'static Mutex<Option<String>> {
    static CACHE: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

fn remember_repo(path: &str) {
    let mut guard = last_known_repo()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *guard = Some(path.to_string());
}

fn read_remembered_repo() -> Option<String> {
    last_known_repo()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

/// Walk upward from `start` looking for the nearest directory that contains a
/// `.git` entry (either a directory or a gitfile, as used by worktrees and
/// submodules). Returns the absolute path of that directory.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn find_repo_ancestor(start: &Path) -> Option<String> {
    let mut current = Some(start.to_path_buf());
    while let Some(dir) = current {
        if dir.join(".git").exists() {
            return Some(dir.to_string_lossy().to_string());
        }
        current = dir.parent().map(Path::to_path_buf);
    }
    None
}

#[cfg(target_os = "linux")]
fn detect_from_active_window() -> Result<Option<String>, GitError> {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt, Window};

    let (conn, screen_num) =
        x11rb::connect(None).map_err(|e| GitError::WindowManager(e.to_string()))?;
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;

    let net_active = conn
        .intern_atom(false, b"_NET_ACTIVE_WINDOW")
        .map_err(|e| GitError::WindowManager(e.to_string()))?
        .reply()
        .map_err(|e| GitError::WindowManager(e.to_string()))?
        .atom;

    let net_wm_pid = conn
        .intern_atom(false, b"_NET_WM_PID")
        .map_err(|e| GitError::WindowManager(e.to_string()))?
        .reply()
        .map_err(|e| GitError::WindowManager(e.to_string()))?
        .atom;

    let active = conn
        .get_property(false, root, net_active, AtomEnum::WINDOW, 0, 1)
        .map_err(|e| GitError::WindowManager(e.to_string()))?
        .reply()
        .map_err(|e| GitError::WindowManager(e.to_string()))?;

    let window = match active.value32().and_then(|v| v.first()) {
        Some(&w) if w != 0 => Window::from(w),
        _ => return Ok(None),
    };

    let pid_prop = conn
        .get_property(false, window, net_wm_pid, AtomEnum::CARDINAL, 0, 1)
        .map_err(|e| GitError::WindowManager(e.to_string()))?
        .reply()
        .map_err(|e| GitError::WindowManager(e.to_string()))?;

    let pid = match pid_prop.value32().and_then(|v| v.first()) {
        Some(&p) => p,
        None => return Ok(None),
    };

    let cwd = std::fs::read_link(format!("/proc/{}/cwd", pid))?;
    Ok(find_repo_ancestor(&cwd))
}

#[cfg(not(target_os = "linux"))]
fn detect_from_active_window() -> Result<Option<String>, GitError> {
    Ok(None)
}

/// Collect the paths of every file that `git status` would report as changed
/// (including untracked files). Results are sorted and de-duplicated.
fn collect_changed_files(repo: &git2::Repository) -> Result<Vec<String>, GitError> {
    let statuses = repo.statuses(None)?;
    let mut files: Vec<String> = statuses
        .iter()
        .filter_map(|entry| entry.path().map(|p| p.to_string()))
        .collect();
    files.sort();
    files.dedup();
    Ok(files)
}

/// Read the current branch name of the repository at `repo_path` via libgit2.
///
/// For a detached HEAD there is no branch name; in that case the full commit
/// id of `HEAD` is returned instead so the caller always has a usable value.
pub fn read_branch(repo_path: &str) -> Result<String, GitError> {
    let repo = git2::Repository::discover(repo_path)?;
    let head = repo.head()?;
    if head.is_branch() {
        Ok(head.shorthand().unwrap_or("HEAD").to_string())
    } else {
        Ok(head.peel_to_commit()?.id().to_string())
    }
}

/// Assemble the full git context for the repository at `repo_path`:
/// working-directory path, current branch, latest commit hash and the list of
/// changed files. `repo_path` may point at the repository root or any nested
/// directory inside it.
pub fn read_git_context(repo_path: &str) -> Result<GitContext, GitError> {
    let repo = git2::Repository::discover(repo_path)?;
    let workdir = repo.workdir().ok_or(GitError::BareRepository)?;
    let branch = read_branch(repo_path)?;
    let commit_hash = repo.head()?.peel_to_commit()?.id().to_string();
    let changed_files = collect_changed_files(&repo)?;

    Ok(GitContext {
        repo_path: workdir.to_string_lossy().to_string(),
        branch,
        commit_hash,
        changed_files,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a temporary git repository with one committed file (`hello.txt`)
    /// and return its path plus the underlying `git2::Repository`.
    fn init_test_repo() -> (std::path::PathBuf, git2::Repository) {
        let mut dir = std::env::temp_dir();
        let unique = format!(
            "parkplatzcoder_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        dir.push(unique);
        std::fs::create_dir_all(&dir).unwrap();

        let repo = git2::Repository::init(&dir).unwrap();
        {
            let mut cfg = repo.config().unwrap();
            cfg.set_str("user.name", "Test").unwrap();
            cfg.set_str("user.email", "test@example.com").unwrap();
        }

        std::fs::write(dir.join("hello.txt"), "hello\n").unwrap();
        commit_all(&repo, "initial commit");

        (dir, repo)
    }

    fn commit_all(repo: &git2::Repository, message: &str) {
        let mut index = repo.index().unwrap();
        index
            .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
            .unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        let parent_commit = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
        let parents: Vec<&git2::Commit> = parent_commit.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
            .unwrap();
    }

    #[test]
    fn read_branch_returns_the_current_branch() {
        let (dir, repo) = init_test_repo();
        let expected = repo.head().unwrap().shorthand().unwrap().to_string();
        let branch = read_branch(dir.to_str().unwrap()).unwrap();
        assert_eq!(branch, expected);
    }

    #[test]
    fn read_git_context_reports_branch_commit_and_changed_files() {
        let (dir, repo) = init_test_repo();

        let head_commit = repo.head().unwrap().peel_to_commit().unwrap().id();
        let expected_branch = repo.head().unwrap().shorthand().unwrap().to_string();

        // Change an existing file and add a new untracked one.
        std::fs::write(dir.join("hello.txt"), "hello, world\n").unwrap();
        std::fs::write(dir.join("world.txt"), "new file\n").unwrap();

        let ctx = read_git_context(dir.to_str().unwrap()).unwrap();

        assert_eq!(ctx.branch, expected_branch);
        assert_eq!(ctx.commit_hash, head_commit.to_string());
        assert_eq!(ctx.changed_files, vec!["hello.txt", "world.txt"]);
    }

    #[test]
    fn read_git_context_works_from_a_nested_directory() {
        let (dir, _repo) = init_test_repo();
        let sub = dir.join("a").join("b");
        std::fs::create_dir_all(&sub).unwrap();

        let ctx = read_git_context(sub.to_str().unwrap()).unwrap();
        assert_eq!(
            std::fs::canonicalize(&ctx.repo_path).unwrap(),
            std::fs::canonicalize(&dir).unwrap()
        );
    }

    #[test]
    fn find_repo_ancestor_walks_up_to_the_repository() {
        let (dir, _repo) = init_test_repo();
        let sub = dir.join("a").join("b");
        std::fs::create_dir_all(&sub).unwrap();

        let found = find_repo_ancestor(&sub).unwrap();
        assert_eq!(found, dir.to_string_lossy().to_string());
    }

    #[test]
    fn find_repo_ancestor_returns_none_outside_any_repo() {
        let mut dir = std::env::temp_dir();
        dir.push(format!("parkplatzcoder_no_git_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(find_repo_ancestor(&dir).is_none());
    }
}
