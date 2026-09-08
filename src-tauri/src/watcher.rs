use tauri::AppHandle;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WatcherError {
    #[error("watcher module not implemented yet")]
    NotImplemented,
}

pub fn start_watcher(_app: AppHandle) -> Result<(), WatcherError> {
    Err(WatcherError::NotImplemented)
}
