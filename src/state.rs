use std::fs;
use std::path::PathBuf;

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
