use crate::model::{NewNote, Note};
use rusqlite::{params, types::Type, Connection};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Directory (relative to the user data dir) that holds all local data.
const DATA_DIR_NAME: &str = "parkplatzcoder";
/// SQLite database file name.
const DB_FILE_NAME: &str = "notes.db";

const SELECT_NOTES: &str =
    "SELECT id, text, repo_path, branch, commit_hash, changed_files, created_at, done FROM notes";

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("could not determine the user data directory")]
    DataDir,
    #[error("could not create the data directory: {0}")]
    CreateDir(#[source] std::io::Error),
    #[error("could not set file permissions: {0}")]
    SetPermissions(#[source] std::io::Error),
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("could not serialize changed files: {0}")]
    Serialize(#[source] serde_json::Error),
}

/// Absolute path of the SQLite database file.
fn db_path() -> Result<PathBuf, StorageError> {
    let base = dirs::data_dir().ok_or(StorageError::DataDir)?;
    Ok(base.join(DATA_DIR_NAME).join(DB_FILE_NAME))
}

/// Initialize the SQLite database (0600 permissions, idempotent).
pub fn init_db() -> Result<(), StorageError> {
    let _conn = open_connection()?;
    Ok(())
}

/// Open (and, if necessary, initialize) the production database.
fn open_connection() -> Result<Connection, StorageError> {
    open_connection_at(&db_path()?)
}

/// Open a connection at an explicit path, creating the directory, applying
/// private permissions and building the schema. Kept separate so tests can
/// point at a temporary location without touching the real user data dir.
fn open_connection_at(path: &Path) -> Result<Connection, StorageError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(StorageError::CreateDir)?;
        apply_dir_permissions(parent)?;
    }
    let conn = Connection::open(path)?;
    apply_file_permissions(path)?;
    init_schema(&conn)?;
    Ok(conn)
}

fn init_schema(conn: &Connection) -> Result<(), StorageError> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS notes (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            text         TEXT NOT NULL,
            repo_path    TEXT NOT NULL,
            branch       TEXT NOT NULL,
            commit_hash  TEXT NOT NULL,
            changed_files TEXT NOT NULL,
            created_at   TEXT NOT NULL,
            done         INTEGER NOT NULL DEFAULT 0
        )",
        (),
    )?;
    Ok(())
}

#[cfg(unix)]
fn apply_file_permissions(path: &Path) -> Result<(), StorageError> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .map_err(StorageError::SetPermissions)
}

#[cfg(not(unix))]
fn apply_file_permissions(_path: &Path) -> Result<(), StorageError> {
    Ok(())
}

#[cfg(unix)]
fn apply_dir_permissions(path: &Path) -> Result<(), StorageError> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
        .map_err(StorageError::SetPermissions)
}

#[cfg(not(unix))]
fn apply_dir_permissions(_path: &Path) -> Result<(), StorageError> {
    Ok(())
}

/// RFC3339 UTC timestamp with millisecond precision, e.g. `2026-09-08T12:34:56.789Z`.
fn now_utc_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn note_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Note> {
    let changed_files_json: String = row.get(5)?;
    let changed_files: Vec<String> = serde_json::from_str(&changed_files_json)
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(5, Type::Text, Box::new(e)))?;
    Ok(Note {
        id: row.get(0)?,
        text: row.get(1)?,
        repo_path: row.get(2)?,
        branch: row.get(3)?,
        commit_hash: row.get(4)?,
        changed_files,
        created_at: row.get(6)?,
        done: row.get::<_, i64>(7)? != 0,
    })
}

fn query_notes<P>(conn: &Connection, sql: &str, params: P) -> Result<Vec<Note>, StorageError>
where
    P: rusqlite::Params,
{
    let mut stmt = conn.prepare(sql)?;
    let mut notes = Vec::new();
    let rows = stmt.query_map(params, note_from_row)?;
    for row in rows {
        notes.push(row?);
    }
    Ok(notes)
}

/// Escape LIKE wildcards so user input is matched literally.
fn escape_like_pattern(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '\\' | '%' | '_' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

pub fn insert_note(note: NewNote) -> Result<Note, StorageError> {
    let conn = open_connection()?;
    insert_note_in(&conn, &note)
}

pub fn list_notes() -> Result<Vec<Note>, StorageError> {
    let conn = open_connection()?;
    list_notes_in(&conn)
}

pub fn search_notes(query: &str) -> Result<Vec<Note>, StorageError> {
    let conn = open_connection()?;
    search_notes_in(&conn, query)
}

pub fn set_note_done(id: i64, done: bool) -> Result<(), StorageError> {
    let conn = open_connection()?;
    set_note_done_in(&conn, id, done)
}

pub fn delete_note(id: i64) -> Result<(), StorageError> {
    let conn = open_connection()?;
    delete_note_in(&conn, id)
}

pub fn latest_note_for_branch(repo: &str, branch: &str) -> Result<Option<Note>, StorageError> {
    let conn = open_connection()?;
    latest_note_for_branch_in(&conn, repo, branch)
}

pub fn distinct_repos() -> Result<Vec<String>, StorageError> {
    let conn = open_connection()?;
    distinct_repos_in(&conn)
}

fn insert_note_in(conn: &Connection, note: &NewNote) -> Result<Note, StorageError> {
    let changed_files_json =
        serde_json::to_string(&note.changed_files).map_err(StorageError::Serialize)?;
    let created_at = now_utc_rfc3339();
    conn.execute(
        "INSERT INTO notes (text, repo_path, branch, commit_hash, changed_files, created_at, done)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)",
        params![
            note.text,
            note.repo_path,
            note.branch,
            note.commit_hash,
            changed_files_json,
            created_at,
        ],
    )?;
    let id = conn.last_insert_rowid();
    Ok(Note {
        id,
        text: note.text.clone(),
        repo_path: note.repo_path.clone(),
        branch: note.branch.clone(),
        commit_hash: note.commit_hash.clone(),
        changed_files: note.changed_files.clone(),
        created_at,
        done: false,
    })
}

fn list_notes_in(conn: &Connection) -> Result<Vec<Note>, StorageError> {
    query_notes(
        conn,
        &format!("{SELECT_NOTES} ORDER BY created_at DESC, id DESC"),
        (),
    )
}

fn search_notes_in(conn: &Connection, query: &str) -> Result<Vec<Note>, StorageError> {
    let pattern = format!("%{}%", escape_like_pattern(query));
    let sql = format!(
        "{SELECT_NOTES} WHERE text LIKE ?1 ESCAPE '\\' OR repo_path LIKE ?2 ESCAPE '\\' \
         OR branch LIKE ?3 ESCAPE '\\' ORDER BY created_at DESC, id DESC"
    );
    query_notes(conn, &sql, params![pattern, pattern, pattern])
}

fn set_note_done_in(conn: &Connection, id: i64, done: bool) -> Result<(), StorageError> {
    conn.execute(
        "UPDATE notes SET done = ?2 WHERE id = ?1",
        params![id, done as i64],
    )?;
    Ok(())
}

fn delete_note_in(conn: &Connection, id: i64) -> Result<(), StorageError> {
    conn.execute("DELETE FROM notes WHERE id = ?1", params![id])?;
    Ok(())
}

fn latest_note_for_branch_in(
    conn: &Connection,
    repo: &str,
    branch: &str,
) -> Result<Option<Note>, StorageError> {
    let sql = format!(
        "{SELECT_NOTES} WHERE repo_path = ?1 AND branch = ?2 \
         ORDER BY created_at DESC, id DESC LIMIT 1"
    );
    let notes = query_notes(conn, &sql, params![repo, branch])?;
    Ok(notes.into_iter().next())
}

fn distinct_repos_in(conn: &Connection) -> Result<Vec<String>, StorageError> {
    let mut stmt = conn.prepare("SELECT DISTINCT repo_path FROM notes ORDER BY repo_path ASC")?;
    let rows = stmt.query_map((), |row| row.get::<_, String>(0))?;
    let mut repos = Vec::new();
    for repo in rows {
        repos.push(repo?);
    }
    Ok(repos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::NewNote;

    fn mem_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        conn
    }

    fn sample_note(repo: &str, branch: &str, text: &str) -> NewNote {
        NewNote {
            text: text.to_string(),
            repo_path: repo.to_string(),
            branch: branch.to_string(),
            commit_hash: "abc123".to_string(),
            changed_files: vec!["src/main.rs".to_string(), "README.md".to_string()],
        }
    }

    #[test]
    fn insert_and_list_roundtrip() {
        let conn = mem_conn();
        let note = insert_note_in(
            &conn,
            &NewNote {
                text: "Zettel eins".to_string(),
                repo_path: "/repo/a".to_string(),
                branch: "main".to_string(),
                commit_hash: "deadbeef".to_string(),
                changed_files: vec!["a.rs".to_string(), "b.rs".to_string()],
            },
        )
        .unwrap();

        assert!(note.id > 0);
        assert!(!note.created_at.is_empty());
        assert!(!note.done);

        let all = list_notes_in(&conn).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, note.id);
        assert_eq!(all[0].text, "Zettel eins");
        assert_eq!(all[0].repo_path, "/repo/a");
        assert_eq!(all[0].branch, "main");
        assert_eq!(all[0].commit_hash, "deadbeef");
        assert_eq!(
            all[0].changed_files,
            vec!["a.rs".to_string(), "b.rs".to_string()]
        );
        assert_eq!(all[0].created_at, note.created_at);
    }

    #[test]
    fn list_orders_by_created_at_descending() {
        let conn = mem_conn();
        let first = insert_note_in(&conn, &sample_note("/r", "main", "erste")).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        let second = insert_note_in(&conn, &sample_note("/r", "main", "zweite")).unwrap();

        let all = list_notes_in(&conn).unwrap();
        assert_eq!(all.len(), 2);
        // Newest first (created_at desc, then id desc).
        assert_eq!(all[0].id, second.id);
        assert_eq!(all[1].id, first.id);
    }

    #[test]
    fn search_matches_text_repo_and_branch() {
        let conn = mem_conn();
        insert_note_in(&conn, &sample_note("/repo/one", "main", "todo kartoffel")).unwrap();
        insert_note_in(&conn, &sample_note("/repo/two", "feat/x", "kein treffer")).unwrap();
        insert_note_in(
            &conn,
            &sample_note("/repo/three", "release", "anderes wort"),
        )
        .unwrap();

        // By text.
        let hits = search_notes_in(&conn, "kartoffel").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].repo_path, "/repo/one");

        // By repo path.
        let hits = search_notes_in(&conn, "/repo/two").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].text, "kein treffer");

        // By branch.
        let hits = search_notes_in(&conn, "release").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].text, "anderes wort");

        // No match.
        assert!(search_notes_in(&conn, "gibtsnicht").unwrap().is_empty());
    }

    #[test]
    fn search_treats_like_wildcards_literally() {
        let conn = mem_conn();
        insert_note_in(&conn, &sample_note("/r", "main", "100% fertig")).unwrap();
        insert_note_in(&conn, &sample_note("/r", "main", "100x fertig")).unwrap();

        // A literal percent sign must not act as a wildcard.
        let hits = search_notes_in(&conn, "100%").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].text, "100% fertig");
    }

    #[test]
    fn set_note_done_flips_flag() {
        let conn = mem_conn();
        let note = insert_note_in(&conn, &sample_note("/r", "main", "abhaken")).unwrap();
        assert!(!note.done);

        set_note_done_in(&conn, note.id, true).unwrap();
        let all = list_notes_in(&conn).unwrap();
        assert_eq!(all.len(), 1);
        assert!(all[0].done);

        set_note_done_in(&conn, note.id, false).unwrap();
        let all = list_notes_in(&conn).unwrap();
        assert!(!all[0].done);
    }

    #[test]
    fn delete_removes_row_physically() {
        let conn = mem_conn();
        let note = insert_note_in(&conn, &sample_note("/r", "main", "loeschen")).unwrap();
        assert_eq!(list_notes_in(&conn).unwrap().len(), 1);

        delete_note_in(&conn, note.id).unwrap();

        assert!(list_notes_in(&conn).unwrap().is_empty());
        // The row and all of its context fields are gone from the table itself.
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM notes WHERE id = ?1",
                params![note.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn latest_note_for_branch_returns_newest() {
        let conn = mem_conn();
        insert_note_in(&conn, &sample_note("/repo/a", "main", "alt")).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        let newest = insert_note_in(&conn, &sample_note("/repo/a", "main", "neu")).unwrap();
        insert_note_in(&conn, &sample_note("/repo/a", "other", "anderer branch")).unwrap();

        let latest = latest_note_for_branch_in(&conn, "/repo/a", "main")
            .unwrap()
            .expect("note should exist");
        assert_eq!(latest.id, newest.id);
        assert_eq!(latest.text, "neu");

        assert!(latest_note_for_branch_in(&conn, "/repo/a", "nicht-da")
            .unwrap()
            .is_none());
    }

    #[test]
    fn distinct_repos_returns_unique_sorted() {
        let conn = mem_conn();
        insert_note_in(&conn, &sample_note("/repo/b", "main", "b")).unwrap();
        insert_note_in(&conn, &sample_note("/repo/a", "main", "a")).unwrap();
        insert_note_in(&conn, &sample_note("/repo/b", "other", "b2")).unwrap();

        let repos = distinct_repos_in(&conn).unwrap();
        assert_eq!(repos, vec!["/repo/a".to_string(), "/repo/b".to_string()]);
    }

    #[cfg(unix)]
    #[test]
    fn db_file_has_0600_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = std::env::temp_dir().join(format!("parkplatzcoder_test_{}", std::process::id()));
        let db = dir.join("notes.db");
        let _ = std::fs::remove_dir_all(&dir);

        {
            let _conn = open_connection_at(&db).unwrap();
        }

        let mode = std::fs::metadata(&db).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
