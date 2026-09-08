use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewNote {
    pub text: String,
    pub repo_path: String,
    pub branch: String,
    pub commit_hash: String,
    pub changed_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: i64,
    pub text: String,
    pub repo_path: String,
    pub branch: String,
    pub commit_hash: String,
    pub changed_files: Vec<String>,
    /// RFC3339 UTC timestamp.
    pub created_at: String,
    pub done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitContext {
    pub repo_path: String,
    pub branch: String,
    pub commit_hash: String,
    pub changed_files: Vec<String>,
}
