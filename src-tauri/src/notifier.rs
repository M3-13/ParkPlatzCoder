use std::path::Path;

use tauri::{AppHandle, Runtime};
use tauri_plugin_notification::NotificationExt;

use crate::model::Note;

#[derive(Debug)]
pub enum NotifyError {
    Show(String),
}

impl std::fmt::Display for NotifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotifyError::Show(msg) => write!(f, "notification failed: {msg}"),
        }
    }
}

impl std::error::Error for NotifyError {}

pub fn notify_branch_note<R: Runtime>(app: &AppHandle<R>, note: &Note) -> Result<(), NotifyError> {
    app.notification()
        .builder()
        .title("ParkPlatzCoder")
        .body(note.text.clone())
        .show()
        .map_err(|e| NotifyError::Show(e.to_string()))
}

pub fn notify_export_done<R: Runtime>(app: &AppHandle<R>, path: &Path) -> Result<(), NotifyError> {
    app.notification()
        .builder()
        .title("Export abgeschlossen")
        .body(path.display().to_string())
        .show()
        .map_err(|e| NotifyError::Show(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notify_error_display_contains_message() {
        let err = NotifyError::Show("boom".to_string());
        assert!(err.to_string().contains("boom"));
    }
}
