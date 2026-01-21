use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

const STALE_SESSION_AGE: Duration = Duration::from_secs(24 * 60 * 60);

/// Get the sessions directory, creating it if it doesn't exist.
fn get_sessions_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let sessions_dir = PathBuf::from(home)
        .join(".cache")
        .join("termtint")
        .join("sessions");
    let _ = fs::create_dir_all(&sessions_dir);
    sessions_dir
}

/// Clean up stale session directories older than STALE_SESSION_AGE.
pub fn cleanup_stale_sessions() {
    let sessions = get_sessions_dir();
    let Ok(entries) = fs::read_dir(&sessions) else { return };

    for entry in entries.flatten() {
        let config_path = entry.path().join("last_config");
        if let Ok(metadata) = fs::metadata(&config_path) {
            if let Ok(modified) = metadata.modified() {
                if SystemTime::now()
                    .duration_since(modified)
                    .map(|age| age > STALE_SESSION_AGE)
                    .unwrap_or(false)
                {
                    let _ = fs::remove_dir_all(entry.path());
                }
            }
        }
    }
}

/// Get the path to the state file.
fn state_file_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".cache")
        .join("termtint")
        .join("last_config")
}

/// Read the last config file path from state, if any.
pub fn read_last_config_path() -> Option<PathBuf> {
    let state_path = state_file_path();
    fs::read_to_string(&state_path)
        .ok()
        .map(|s| PathBuf::from(s.trim()))
        .filter(|p| !p.as_os_str().is_empty())
}

/// Write the current config file path to state.
/// Pass None to clear the state (when leaving a termtint project).
pub fn write_last_config_path(path: Option<&std::path::Path>) {
    let state_path = state_file_path();

    // Ensure parent directory exists
    if let Some(parent) = state_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    match path {
        Some(p) => {
            let _ = fs::write(&state_path, p.to_string_lossy().as_bytes());
        }
        None => {
            let _ = fs::remove_file(&state_path);
        }
    }
}
