use crate::model::{NewNote, Note};
use crate::{export, git_context, storage};

#[tauri::command]
pub fn park_note(text: String) -> Result<Note, String> {
    let repo_path = git_context::detect_repo()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no git repository detected".to_string())?;
    let ctx = git_context::read_git_context(&repo_path).map_err(|e| e.to_string())?;
    let new_note = NewNote {
        text,
        repo_path,
        branch: ctx.branch,
        commit_hash: ctx.commit_hash,
        changed_files: ctx.changed_files,
    };
    storage::insert_note(new_note).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_notes() -> Result<Vec<Note>, String> {
    storage::list_notes().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn search_notes(query: String) -> Result<Vec<Note>, String> {
    storage::search_notes(&query).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn toggle_done(id: i64) -> Result<(), String> {
    storage::set_note_done(id, true).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_note(id: i64) -> Result<(), String> {
    storage::delete_note(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_json() -> Result<String, String> {
    export::export_all_notes()
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(|e| e.to_string())
}
