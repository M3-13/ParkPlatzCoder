use parkplatzcoder::model::{GitContext, NewNote, Note};
use parkplatzcoder::{export, git_context, notifier, storage, watcher};
use std::path::{Path, PathBuf};

#[test]
fn run_entry_point_present() {
    let _: fn() = parkplatzcoder::run;
}

#[test]
fn note_roundtrips_through_json() {
    let note = Note {
        id: 7,
        text: "Refactor eingabefeld".to_string(),
        repo_path: "/home/dev/repo".to_string(),
        branch: "main".to_string(),
        commit_hash: "abc123def456".to_string(),
        changed_files: vec!["src/main.rs".to_string(), "README.md".to_string()],
        created_at: "2026-09-08T10:00:00Z".to_string(),
        done: false,
    };

    let json = serde_json::to_string(&note).unwrap();
    let back: Note = serde_json::from_str(&json).unwrap();

    assert_eq!(back.id, 7);
    assert_eq!(back.text, "Refactor eingabefeld");
    assert_eq!(back.repo_path, "/home/dev/repo");
    assert_eq!(back.branch, "main");
    assert_eq!(back.commit_hash, "abc123def456");
    assert_eq!(back.changed_files.len(), 2);
    assert_eq!(back.created_at, "2026-09-08T10:00:00Z");
    assert!(!back.done);
}

#[test]
fn new_note_roundtrips_through_json() {
    let new_note = NewNote {
        text: "x".to_string(),
        repo_path: "/r".to_string(),
        branch: "feat/x".to_string(),
        commit_hash: "deadbeef".to_string(),
        changed_files: vec![],
    };

    let back: NewNote = serde_json::from_str(&serde_json::to_string(&new_note).unwrap()).unwrap();
    assert_eq!(back.branch, "feat/x");
    assert!(back.changed_files.is_empty());
}

#[test]
fn git_context_roundtrips_through_json() {
    let ctx = GitContext {
        repo_path: "/r".to_string(),
        branch: "main".to_string(),
        commit_hash: "c0ffee".to_string(),
        changed_files: vec!["a.txt".to_string()],
    };

    let back: GitContext = serde_json::from_str(&serde_json::to_string(&ctx).unwrap()).unwrap();
    assert_eq!(back.commit_hash, "c0ffee");
    assert_eq!(back.changed_files, vec!["a.txt".to_string()]);
}

// The following tests assert only that the stub functions exist with the exact
// signatures the shared sprint contract promises. They deliberately do NOT call
// the stubs, so they hold before and after the owning tickets fill the bodies.

#[test]
fn storage_stub_signatures_present() {
    let _: fn() -> Result<(), storage::StorageError> = storage::init_db;
    let _: fn(NewNote) -> Result<Note, storage::StorageError> = storage::insert_note;
    let _: fn() -> Result<Vec<Note>, storage::StorageError> = storage::list_notes;
    let _: fn(&str) -> Result<Vec<Note>, storage::StorageError> = storage::search_notes;
    let _: fn(i64, bool) -> Result<(), storage::StorageError> = storage::set_note_done;
    let _: fn(i64) -> Result<(), storage::StorageError> = storage::delete_note;
    let _: fn(&str, &str) -> Result<Option<Note>, storage::StorageError> =
        storage::latest_note_for_branch;
    let _: fn() -> Result<Vec<String>, storage::StorageError> = storage::distinct_repos;
}

#[test]
fn git_context_stub_signatures_present() {
    let _: fn() -> Result<Option<String>, git_context::GitError> = git_context::detect_repo;
    let _: fn(&str) -> Result<GitContext, git_context::GitError> = git_context::read_git_context;
    let _: fn(&str) -> Result<String, git_context::GitError> = git_context::read_branch;
}

#[test]
fn export_stub_signature_present() {
    let _: fn() -> Result<PathBuf, export::ExportError> = export::export_all_notes;
}

#[test]
fn watcher_stub_signature_present() {
    let _: fn(tauri::AppHandle) -> Result<(), watcher::WatcherError> = watcher::start_watcher;
}

#[test]
fn notifier_stub_signatures_present() {
    let _: fn(&tauri::AppHandle, &Note) -> Result<(), notifier::NotifyError> =
        notifier::notify_branch_note;
    let _: fn(&tauri::AppHandle, &Path) -> Result<(), notifier::NotifyError> =
        notifier::notify_export_done;
}
