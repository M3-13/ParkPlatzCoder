use crate::model::Note;
use std::path::Path;
use tauri::AppHandle;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NotifyError {
    #[error("notifier module not implemented yet")]
    NotImplemented,
}

pub fn notify_branch_note(_app: &AppHandle, _note: &Note) -> Result<(), NotifyError> {
    Err(NotifyError::NotImplemented)
}

pub fn notify_export_done(_app: &AppHandle, _path: &Path) -> Result<(), NotifyError> {
    Err(NotifyError::NotImplemented)
}
