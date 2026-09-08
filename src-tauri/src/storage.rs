use crate::model::{NewNote, Note};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("storage module not implemented yet")]
    NotImplemented,
}

/// Initialize the SQLite database (0600 permissions, idempotent).
pub fn init_db() -> Result<(), StorageError> {
    Err(StorageError::NotImplemented)
}

pub fn insert_note(_note: NewNote) -> Result<Note, StorageError> {
    Err(StorageError::NotImplemented)
}

/// List all notes ordered by `created_at` descending.
pub fn list_notes() -> Result<Vec<Note>, StorageError> {
    Err(StorageError::NotImplemented)
}

/// LIKE search over text/repo/branch.
pub fn search_notes(_query: &str) -> Result<Vec<Note>, StorageError> {
    Err(StorageError::NotImplemented)
}

pub fn set_note_done(_id: i64, _done: bool) -> Result<(), StorageError> {
    Err(StorageError::NotImplemented)
}

/// Physically delete a note.
pub fn delete_note(_id: i64) -> Result<(), StorageError> {
    Err(StorageError::NotImplemented)
}

pub fn latest_note_for_branch(_repo: &str, _branch: &str) -> Result<Option<Note>, StorageError> {
    Err(StorageError::NotImplemented)
}

pub fn distinct_repos() -> Result<Vec<String>, StorageError> {
    Err(StorageError::NotImplemented)
}
