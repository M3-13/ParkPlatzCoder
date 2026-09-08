use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use thiserror::Error;

use crate::model::Note;
use crate::storage;

#[derive(Debug, Error)]
pub enum ExportError {
    #[error("could not determine the user data directory")]
    NoDataDir,
    #[error("could not create the export directory: {0}")]
    CreateDir(#[source] std::io::Error),
    #[error("storage error: {0}")]
    Storage(#[from] storage::StorageError),
    #[error("serialization error: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Directory below the user data dir that holds the notes database and exports.
const DATA_DIR_NAME: &str = "parkplatzcoder";

/// Build the absolute export file path:
/// `<data_dir>/parkplatzcoder/export-<UTC-timestamp>.json`.
fn export_path(timestamp: &str) -> Result<PathBuf, ExportError> {
    let data_dir = dirs::data_dir().ok_or(ExportError::NoDataDir)?;
    Ok(data_dir
        .join(DATA_DIR_NAME)
        .join(format!("export-{}.json", timestamp)))
}

/// Current UTC time formatted for a filename-safe timestamp
/// (e.g. `20260908T123456Z`).
fn utc_timestamp() -> String {
    chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string()
}

/// Serialize the notes into a pretty-printed JSON document.
fn serialize_notes(notes: &[Note]) -> Result<String, ExportError> {
    Ok(serde_json::to_string_pretty(notes)?)
}

/// Write `contents` to `path`, creating the parent directory if needed, and set
/// Unix mode 0600 on the result. On non-Unix platforms the permission step is a
/// no-op.
fn write_private(path: &PathBuf, contents: &str) -> Result<(), ExportError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(ExportError::CreateDir)?;
    }

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)?;
    file.write_all(contents.as_bytes())?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(0o600))?;
    }

    Ok(())
}

/// Export all notes as JSON (file written with 0600 permissions).
pub fn export_all_notes() -> Result<PathBuf, ExportError> {
    let notes = storage::list_notes()?;
    let contents = serialize_notes(&notes)?;
    let path = export_path(&utc_timestamp())?;
    write_private(&path, &contents)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_note() -> Note {
        Note {
            id: 1,
            text: "einkaufen".to_string(),
            repo_path: "/home/user/project".to_string(),
            branch: "main".to_string(),
            commit_hash: "abc123def456".to_string(),
            changed_files: vec!["src/main.rs".to_string(), "Cargo.toml".to_string()],
            created_at: "2026-09-08T12:34:56Z".to_string(),
            done: false,
        }
    }

    #[test]
    fn serialize_contains_all_context_fields() {
        let notes = vec![sample_note()];
        let json = serialize_notes(&notes).expect("serialization must succeed");
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("output must be valid JSON");

        let note = &value[0];
        for field in [
            "id",
            "text",
            "repo_path",
            "branch",
            "commit_hash",
            "changed_files",
            "created_at",
            "done",
        ] {
            assert!(
                note.get(field).is_some(),
                "missing context field `{}` in export",
                field
            );
        }
    }

    #[test]
    fn serialize_roundtrips_through_model() {
        let notes = vec![sample_note()];
        let json = serialize_notes(&notes).expect("serialization must succeed");
        let back: Vec<Note> = serde_json::from_str(&json).expect("deserialization must succeed");
        assert_eq!(back.len(), notes.len());

        let (orig, round) = (&notes[0], &back[0]);
        assert_eq!(round.id, orig.id);
        assert_eq!(round.text, orig.text);
        assert_eq!(round.repo_path, orig.repo_path);
        assert_eq!(round.branch, orig.branch);
        assert_eq!(round.commit_hash, orig.commit_hash);
        assert_eq!(round.changed_files, orig.changed_files);
        assert_eq!(round.created_at, orig.created_at);
        assert_eq!(round.done, orig.done);
    }

    #[test]
    fn utc_timestamp_matches_expected_format() {
        let ts = utc_timestamp();
        // Length: 8 digits (date) + 'T' + 6 digits (time) + 'Z'.
        assert_eq!(ts.len(), 16, "unexpected timestamp length: {}", ts);
        assert!(ts.ends_with('Z'), "timestamp must be UTC: {}", ts);
        assert!(
            ts.chars()
                .all(|c| c.is_ascii_digit() || c == 'T' || c == 'Z'),
            "timestamp must contain only digits/T/Z: {}",
            ts
        );
    }

    #[cfg(unix)]
    #[test]
    fn write_private_sets_mode_0600() {
        use std::os::unix::fs::PermissionsExt;

        let dir =
            std::env::temp_dir().join(format!("parkplatzcoder-export-test-{}", std::process::id()));
        let path = dir.join("export.json");
        write_private(&path, "{}").expect("write must succeed");

        let metadata = fs::metadata(&path).expect("file must exist");
        assert_eq!(metadata.permissions().mode() & 0o777, 0o600);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn export_path_is_below_data_dir_with_timestamp() {
        let path = export_path("20260908T123456Z").expect("path must build");
        let rendered = path.to_string_lossy().to_string();
        assert!(
            rendered.ends_with("parkplatzcoder/export-20260908T123456Z.json")
                || rendered.ends_with("parkplatzcoder\\export-20260908T123456Z.json"),
            "unexpected path: {}",
            rendered
        );
        assert!(rendered.contains(DATA_DIR_NAME));
    }
}
