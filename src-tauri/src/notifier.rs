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
        .body(branch_note_body(note))
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

fn branch_note_body(note: &Note) -> String {
    note.text.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_note(text: &str) -> crate::model::Note {
        crate::model::Note {
            id: 1,
            text: text.to_string(),
            repo_path: "/repo".to_string(),
            branch: "feature/x".to_string(),
            commit_hash: "abc123".to_string(),
            changed_files: vec![],
            created_at: "2026-09-08T00:00:00Z".to_string(),
            done: false,
        }
    }

    #[test]
    fn notify_error_display_contains_message() {
        let err = NotifyError::Show("boom".to_string());
        assert!(err.to_string().contains("boom"));
    }

    #[test]
    fn branch_note_body_is_plain_note_text() {
        let note = test_note("parke mich");
        assert_eq!(branch_note_body(&note), "parke mich");
    }
}
