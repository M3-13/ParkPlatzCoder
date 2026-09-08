use std::io::Write;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::model::Note;
use crate::storage;

/// Errors that can occur while exporting all notes to a JSON file.
#[derive(Debug, Error)]
pub enum ExportError {
    #[error("failed to list notes: {0}")]
    List(storage::StorageError),
    #[error("could not determine the user data directory")]
    DataDir,
    #[error("failed to create the export directory: {0}")]
    CreateDir(std::io::Error),
    #[error("failed to serialize notes to JSON: {0}")]
    Serialize(serde_json::Error),
    #[error("failed to write the export file: {0}")]
    Write(std::io::Error),
    #[error("failed to set file permissions: {0}")]
    Permissions(std::io::Error),
}

/// Export all notes as JSON (file written with 0600 permissions).
///
/// Loads every note via `storage::list_notes`, serializes them including all
/// context fields and writes the result to
/// `dirs::data_dir()/parkplatzcoder/export-<UTC-Zeitstempel>.json`.
pub fn export_all_notes() -> Result<PathBuf, ExportError> {
    let notes = storage::list_notes().map_err(ExportError::List)?;
    let path = export_path()?;
    write_notes_to_file(&notes, &path)?;
    Ok(path)
}

/// Computes the export path and makes sure the parent directory exists.
fn export_path() -> Result<PathBuf, ExportError> {
    let dir = dirs::data_dir().ok_or(ExportError::DataDir)?;
    let dir = dir.join("parkplatzcoder");
    std::fs::create_dir_all(&dir).map_err(ExportError::CreateDir)?;
    Ok(dir.join(format!("export-{}.json", timestamp_stamp())))
}

/// Current UTC time as a filesystem-safe timestamp, e.g. `20260908T101500.123Z`.
fn timestamp_stamp() -> String {
    chrono::Utc::now().format("%Y%m%dT%H%M%S%.3fZ").to_string()
}

/// Serializes `notes` to pretty JSON and writes them to `path`, restricting the
/// file to the current user (Unix mode 0600).
fn write_notes_to_file(notes: &[Note], path: &Path) -> Result<(), ExportError> {
    let json = serde_json::to_string_pretty(notes).map_err(ExportError::Serialize)?;

    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(ExportError::Write)?;
    file.write_all(json.as_bytes())
        .map_err(ExportError::Write)?;
    file.write_all(b"\n").map_err(ExportError::Write)?;
    file.flush().map_err(ExportError::Write)?;

    // Enforce 0600 even when an existing file with the same name is truncated.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .map_err(ExportError::Permissions)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_notes() -> Vec<Note> {
        vec![
            Note {
                id: 1,
                text: "Erster Zettel".to_string(),
                repo_path: "/home/dev/repo".to_string(),
                branch: "main".to_string(),
                commit_hash: "abc123def456".to_string(),
                changed_files: vec!["src/main.rs".to_string(), "README.md".to_string()],
                created_at: "2026-09-08T10:00:00Z".to_string(),
                done: false,
            },
            Note {
                id: 2,
                text: "Abgehakter Zettel".to_string(),
                repo_path: "/home/dev/other".to_string(),
                branch: "feat/x".to_string(),
                commit_hash: "deadbeef".to_string(),
                changed_files: vec![],
                created_at: "2026-09-08T11:30:00Z".to_string(),
                done: true,
            },
        ]
    }

    #[test]
    fn timestamp_stamp_is_filesystem_safe_utc() {
        let stamp = timestamp_stamp();
        // e.g. 20260908T101500.123Z — UTC, no colon or slash, ends with Z.
        assert_eq!(stamp.len(), 20);
        assert!(stamp.ends_with('Z'));
        assert!(!stamp.contains(':'));
        assert!(!stamp.contains('/'));
        assert!(stamp.contains('T'));
    }

    #[test]
    fn export_serializes_every_context_field() {
        let dir =
            std::env::temp_dir().join(format!("parkplatzcoder-export-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("export.json");

        write_notes_to_file(&sample_notes(), &path).unwrap();

        let raw = std::fs::read_to_string(&path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap();
        let arr = parsed.as_array().unwrap();
        assert_eq!(arr.len(), 2);

        let first = &arr[0];
        assert_eq!(first["id"], 1);
        assert_eq!(first["text"], "Erster Zettel");
        assert_eq!(first["repo_path"], "/home/dev/repo");
        assert_eq!(first["branch"], "main");
        assert_eq!(first["commit_hash"], "abc123def456");
        assert_eq!(
            first["changed_files"],
            serde_json::json!(["src/main.rs", "README.md"])
        );
        assert_eq!(first["created_at"], "2026-09-08T10:00:00Z");
        assert_eq!(first["done"], false);

        let second = &arr[1];
        assert_eq!(second["id"], 2);
        assert_eq!(second["changed_files"], serde_json::json!([]));
        assert_eq!(second["created_at"], "2026-09-08T11:30:00Z");
        assert_eq!(second["done"], true);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn export_roundtrips_through_the_note_model() {
        let dir = std::env::temp_dir().join(format!(
            "parkplatzcoder-export-rt-test-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("export.json");

        write_notes_to_file(&sample_notes(), &path).unwrap();

        let raw = std::fs::read_to_string(&path).unwrap();
        let back: Vec<Note> = serde_json::from_str(&raw).unwrap();
        assert_eq!(back.len(), 2);
        assert_eq!(back[0].id, 1);
        assert_eq!(back[0].text, "Erster Zettel");
        assert_eq!(back[0].repo_path, "/home/dev/repo");
        assert_eq!(back[0].branch, "main");
        assert_eq!(back[0].commit_hash, "abc123def456");
        assert_eq!(back[0].changed_files.len(), 2);
        assert_eq!(back[0].created_at, "2026-09-08T10:00:00Z");
        assert!(!back[0].done);
        assert!(back[1].done);
        assert!(back[1].changed_files.is_empty());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[cfg(unix)]
    #[test]
    fn export_file_is_created_with_0600_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = std::env::temp_dir().join(format!(
            "parkplatzcoder-export-perm-test-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("export.json");

        write_notes_to_file(&sample_notes(), &path).unwrap();

        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);

        std::fs::remove_dir_all(&dir).ok();
    }
}
