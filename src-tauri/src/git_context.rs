use crate::model::GitContext;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GitError {
    #[error("git context module not implemented yet")]
    NotImplemented,
}

/// Returns the absolute path of the enclosing repository, if any.
pub fn detect_repo() -> Result<Option<String>, GitError> {
    Err(GitError::NotImplemented)
}

pub fn read_git_context(_repo_path: &str) -> Result<GitContext, GitError> {
    Err(GitError::NotImplemented)
}

/// Read the current branch via libgit2.
pub fn read_branch(_repo_path: &str) -> Result<String, GitError> {
    Err(GitError::NotImplemented)
}
