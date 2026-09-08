use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExportError {
    #[error("export module not implemented yet")]
    NotImplemented,
}

/// Export all notes as JSON (file written with 0600 permissions).
pub fn export_all_notes() -> Result<PathBuf, ExportError> {
    Err(ExportError::NotImplemented)
}
