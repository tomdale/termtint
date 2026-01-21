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

/// State info for the last applied config.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigState {
    pub path: PathBuf,
    pub mtime: u64,
}

/// Read the last config state from disk, if any.
pub fn read_last_config_state() -> Option<ConfigState> {
    let state_path = state_file_path();
    let content = fs::read_to_string(&state_path).ok()?;
    let mut lines = content.lines();
    let path = PathBuf::from(lines.next()?.trim());
    let mtime = lines.next()?.trim().parse().ok()?;
    if path.as_os_str().is_empty() {
        return None;
    }
    Some(ConfigState { path, mtime })
}

/// Get the modification time of a file as seconds since epoch.
pub fn get_file_mtime(path: &std::path::Path) -> Option<u64> {
    fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs())
}

/// Write the current config state to disk.
/// Pass None to clear the state (when leaving a termtint project).
pub fn write_last_config_state(state: Option<&ConfigState>) {
    let state_path = state_file_path();

    // Ensure parent directory exists
    if let Some(parent) = state_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    match state {
        Some(s) => {
            let content = format!("{}\n{}", s.path.to_string_lossy(), s.mtime);
            let _ = fs::write(&state_path, content.as_bytes());
        }
        None => {
            let _ = fs::remove_file(&state_path);
        }
    }
}
