use std::fs;
use std::path::{Path, PathBuf};

use crate::model::{NewNote, Note};
use rusqlite::{params, Connection};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("cannot determine the user data directory")]
    NoDataDir,
    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("invalid changed_files data: {0}")]
    Json(#[from] serde_json::Error),
}

const SCHEMA: &str = "\
CREATE TABLE IF NOT EXISTS notes (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    text          TEXT NOT NULL,
    repo_path     TEXT NOT NULL,
    branch        TEXT NOT NULL,
    commit_hash   TEXT NOT NULL,
    changed_files TEXT NOT NULL,
    created_at    TEXT NOT NULL,
    done          INTEGER NOT NULL DEFAULT 0
);";

/// Data directory base (`dirs::data_dir()`).
fn data_dir() -> Result<PathBuf, StorageError> {
    dirs::data_dir().ok_or(StorageError::NoDataDir)
}

/// Absolute path of the SQLite database file.
fn db_path() -> Result<PathBuf, StorageError> {
    Ok(data_dir()?.join("parkplatzcoder").join("notes.db"))
}

/// Create the parent directory (idempotent) and, on Unix, restrict it to the
/// running user (0700).
fn ensure_dir_private(dir: &Path) -> Result<(), StorageError> {
    fs::create_dir_all(dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

/// On Unix, restrict the database file to the running user (0600).
fn ensure_file_private(path: &Path) -> Result<(), StorageError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

/// Open (creating if needed) the database at `path`, apply private permissions
/// and ensure the schema exists. Idempotent.
fn connect_at(path: &Path) -> Result<Connection, StorageError> {
    if let Some(parent) = path.parent() {
        ensure_dir_private(parent)?;
    }
    let conn = Connection::open(path)?;
    ensure_file_private(path)?;
    conn.execute_batch(SCHEMA)?;
    Ok(conn)
}

/// Open the production database under `dirs::data_dir()/parkplatzcoder/notes.db`.
fn connect() -> Result<Connection, StorageError> {
    connect_at(&db_path()?)
}

/// Initialize the SQLite database (0600 permissions, idempotent).
pub fn init_db() -> Result<(), StorageError> {
    let _conn = connect()?;
    Ok(())
}

fn note_from_row(row: &rusqlite::Row) -> Result<Note, StorageError> {
    let changed_files_json: String = row.get("changed_files")?;
    let changed_files: Vec<String> = serde_json::from_str(&changed_files_json)?;
    Ok(Note {
        id: row.get("id")?,
        text: row.get("text")?,
        repo_path: row.get("repo_path")?,
        branch: row.get("branch")?,
        commit_hash: row.get("commit_hash")?,
        changed_files,
        created_at: row.get("created_at")?,
        done: row.get("done")?,
    })
}

fn query_notes<P: rusqlite::Params>(
    conn: &Connection,
    sql: &str,
    params: P,
) -> Result<Vec<Note>, StorageError> {
    let mut stmt = conn.prepare(sql)?;
    let mut rows = stmt.query(params)?;
    let mut notes = Vec::new();
    while let Some(row) = rows.next()? {
        notes.push(note_from_row(row)?);
    }
    Ok(notes)
}

fn insert_note_in(conn: &Connection, note: NewNote) -> Result<Note, StorageError> {
    let created_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Nanos, true);
    let changed_files_json = serde_json::to_string(&note.changed_files)?;
    conn.execute(
        "INSERT INTO notes (text, repo_path, branch, commit_hash, changed_files, created_at, done) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)",
        params![
            note.text,
            note.repo_path,
            note.branch,
            note.commit_hash,
            changed_files_json,
            created_at
        ],
    )?;
    Ok(Note {
        id: conn.last_insert_rowid(),
        text: note.text,
        repo_path: note.repo_path,
        branch: note.branch,
        commit_hash: note.commit_hash,
        changed_files: note.changed_files,
        created_at,
        done: false,
    })
}

fn list_notes_in(conn: &Connection) -> Result<Vec<Note>, StorageError> {
    query_notes(
        conn,
        "SELECT id, text, repo_path, branch, commit_hash, changed_files, created_at, done \
         FROM notes ORDER BY created_at DESC, id DESC",
        params![],
    )
}

fn search_notes_in(conn: &Connection, query: &str) -> Result<Vec<Note>, StorageError> {
    let pattern = format!("%{}%", query);
    query_notes(
        conn,
        "SELECT id, text, repo_path, branch, commit_hash, changed_files, created_at, done \
         FROM notes \
         WHERE text LIKE ?1 OR repo_path LIKE ?1 OR branch LIKE ?1 \
         ORDER BY created_at DESC, id DESC",
        params![pattern],
    )
}

fn set_note_done_in(conn: &Connection, id: i64, done: bool) -> Result<(), StorageError> {
    conn.execute(
        "UPDATE notes SET done = ?1 WHERE id = ?2",
        params![done, id],
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
    let mut notes = query_notes(
        conn,
        "SELECT id, text, repo_path, branch, commit_hash, changed_files, created_at, done \
         FROM notes \
         WHERE repo_path = ?1 AND branch = ?2 \
         ORDER BY created_at DESC, id DESC LIMIT 1",
        params![repo, branch],
    )?;
    Ok(notes.pop())
}

fn distinct_repos_in(conn: &Connection) -> Result<Vec<String>, StorageError> {
    let mut stmt = conn.prepare("SELECT DISTINCT repo_path FROM notes ORDER BY repo_path ASC")?;
    let mut rows = stmt.query(params![])?;
    let mut repos = Vec::new();
    while let Some(row) = rows.next()? {
        repos.push(row.get(0)?);
    }
    Ok(repos)
}

pub fn insert_note(note: NewNote) -> Result<Note, StorageError> {
    let conn = connect()?;
    insert_note_in(&conn, note)
}

/// List all notes ordered by `created_at` descending.
pub fn list_notes() -> Result<Vec<Note>, StorageError> {
    let conn = connect()?;
    list_notes_in(&conn)
}

/// LIKE search over text/repo/branch.
pub fn search_notes(query: &str) -> Result<Vec<Note>, StorageError> {
    let conn = connect()?;
    search_notes_in(&conn, query)
}

pub fn set_note_done(id: i64, done: bool) -> Result<(), StorageError> {
    let conn = connect()?;
    set_note_done_in(&conn, id, done)
}

/// Physically delete a note.
pub fn delete_note(id: i64) -> Result<(), StorageError> {
    let conn = connect()?;
    delete_note_in(&conn, id)
}

pub fn latest_note_for_branch(repo: &str, branch: &str) -> Result<Option<Note>, StorageError> {
    let conn = connect()?;
    latest_note_for_branch_in(&conn, repo, branch)
}

pub fn distinct_repos() -> Result<Vec<String>, StorageError> {
    let conn = connect()?;
    distinct_repos_in(&conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let n = COUNTER.fetch_add(1, Ordering::SeqCst);
            let dir = std::env::temp_dir().join(format!(
                "ppc_test_{}_{}_{}",
                name,
                std::process::id(),
                n
            ));
            fs::create_dir_all(&dir).unwrap();
            TempDir(dir)
        }

        fn db_path(&self) -> PathBuf {
            self.0.join("notes.db")
        }

        fn connect(&self) -> Connection {
            connect_at(&self.db_path()).unwrap()
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn new_note(text: &str, repo: &str, branch: &str) -> NewNote {
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
        let t = TempDir::new("roundtrip");
        let conn = t.connect();

        let note = insert_note_in(
            &conn,
            new_note("Refactor eingabefeld", "/home/dev/repo", "main"),
        )
        .unwrap();

        assert!(note.id > 0);
        assert!(!note.done);
        assert!(note.created_at.ends_with('Z'));
        assert!(chrono::DateTime::parse_from_rfc3339(&note.created_at).is_ok());
        assert_eq!(note.changed_files, vec!["src/main.rs", "README.md"]);

        let all = list_notes_in(&conn).unwrap();
        assert_eq!(all.len(), 1);
        let stored = &all[0];
        assert_eq!(stored.id, note.id);
        assert_eq!(stored.text, "Refactor eingabefeld");
        assert_eq!(stored.repo_path, "/home/dev/repo");
        assert_eq!(stored.branch, "main");
        assert_eq!(stored.commit_hash, "abc123");
        assert_eq!(stored.changed_files, vec!["src/main.rs", "README.md"]);
        assert!(!stored.done);
        assert_eq!(stored.created_at, note.created_at);
    }

    #[test]
    fn list_orders_by_created_at_descending() {
        let t = TempDir::new("order");
        let conn = t.connect();

        let first = insert_note_in(&conn, new_note("first", "/r", "main")).unwrap();
        let second = insert_note_in(&conn, new_note("second", "/r", "main")).unwrap();

        let all = list_notes_in(&conn).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].id, second.id);
        assert_eq!(all[1].id, first.id);
    }

    #[test]
    fn search_matches_text_repo_and_branch() {
        let t = TempDir::new("search");
        let conn = t.connect();

        insert_note_in(&conn, new_note("park todo", "/alpha", "main")).unwrap();
        insert_note_in(&conn, new_note("other", "/beta", "feat-x")).unwrap();

        let by_text = search_notes_in(&conn, "todo").unwrap();
        assert_eq!(by_text.len(), 1);
        assert_eq!(by_text[0].repo_path, "/alpha");

        let by_repo = search_notes_in(&conn, "/beta").unwrap();
        assert_eq!(by_repo.len(), 1);
        assert_eq!(by_repo[0].branch, "feat-x");

        let by_branch = search_notes_in(&conn, "feat").unwrap();
        assert_eq!(by_branch.len(), 1);
        assert_eq!(by_branch[0].text, "other");

        assert!(search_notes_in(&conn, "does-not-exist").unwrap().is_empty());
    }

    #[test]
    fn set_done_toggles_flag() {
        let t = TempDir::new("done");
        let conn = t.connect();

        let note = insert_note_in(&conn, new_note("x", "/r", "main")).unwrap();
        assert!(!list_notes_in(&conn).unwrap()[0].done);

        set_note_done_in(&conn, note.id, true).unwrap();
        assert!(list_notes_in(&conn).unwrap()[0].done);

        set_note_done_in(&conn, note.id, false).unwrap();
        assert!(!list_notes_in(&conn).unwrap()[0].done);
    }

    #[test]
    fn delete_removes_row_physically() {
        let t = TempDir::new("delete");
        let conn = t.connect();

        let note = insert_note_in(&conn, new_note("x", "/r", "main")).unwrap();
        assert_eq!(list_notes_in(&conn).unwrap().len(), 1);

        delete_note_in(&conn, note.id).unwrap();
        assert!(list_notes_in(&conn).unwrap().is_empty());

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM notes", params![], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn latest_note_for_branch_returns_newest() {
        let t = TempDir::new("latest");
        let conn = t.connect();

        insert_note_in(&conn, new_note("first", "/r", "main")).unwrap();
        let second = insert_note_in(&conn, new_note("second", "/r", "main")).unwrap();
        insert_note_in(&conn, new_note("other", "/r", "other")).unwrap();

        let latest = latest_note_for_branch_in(&conn, "/r", "main")
            .unwrap()
            .unwrap();
        assert_eq!(latest.id, second.id);
        assert_eq!(latest.text, "second");

        assert!(latest_note_for_branch_in(&conn, "/r", "nope")
            .unwrap()
            .is_none());
        assert!(latest_note_for_branch_in(&conn, "/unknown", "main")
            .unwrap()
            .is_none());
    }

    #[test]
    fn distinct_repos_lists_unique_sorted() {
        let t = TempDir::new("repos");
        let conn = t.connect();

        insert_note_in(&conn, new_note("a", "/beta", "main")).unwrap();
        insert_note_in(&conn, new_note("b", "/alpha", "main")).unwrap();
        insert_note_in(&conn, new_note("c", "/alpha", "feat")).unwrap();

        assert_eq!(
            distinct_repos_in(&conn).unwrap(),
            vec!["/alpha".to_string(), "/beta".to_string()]
        );
    }

    #[cfg(unix)]
    #[test]
    fn db_file_has_0600_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let t = TempDir::new("perms");
        let _conn = t.connect();

        let mode = fs::metadata(t.db_path()).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }
}
