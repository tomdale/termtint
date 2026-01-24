use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const STALE_SESSION_AGE: Duration = Duration::from_secs(24 * 60 * 60);

/// Get the sessions directory path for a given home directory.
fn sessions_dir_for_home(home: &Path) -> PathBuf {
    home.join(".cache").join("termtint").join("sessions")
}

/// Get the sessions directory, creating it if it doesn't exist.
fn get_sessions_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let sessions_dir = sessions_dir_for_home(Path::new(&home));
    let _ = fs::create_dir_all(&sessions_dir);
    sessions_dir
}

/// Clean up stale session directories older than STALE_SESSION_AGE.
pub fn cleanup_stale_sessions() {
    let sessions = get_sessions_dir();
    cleanup_stale_sessions_in(&sessions);
}

/// Clean up stale session directories in the given sessions directory.
fn cleanup_stale_sessions_in(sessions_dir: &Path) {
    let Ok(entries) = fs::read_dir(sessions_dir) else {
        return;
    };

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

/// Get the state file path for a given home directory.
fn state_file_path_for_home(home: &Path) -> PathBuf {
    home.join(".cache").join("termtint").join("last_config")
}

/// Get the path to the state file.
pub fn state_file_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    state_file_path_for_home(Path::new(&home))
}

/// Type of config source.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigSourceType {
    Explicit,    // .termtint file found
    TriggerPath, // Directory matching a trigger path pattern (auto color)
    TriggerFile, // Directory with trigger file (auto color)
}

/// State info for the last applied config.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigState {
    pub path: PathBuf,
    pub mtime: u64,
    pub source_type: ConfigSourceType,
}

/// Read the last config state from disk, if any.
pub fn read_last_config_state() -> Option<ConfigState> {
    read_last_config_state_from(&state_file_path())
}

/// Read the last config state from a specific file path.
fn read_last_config_state_from(state_path: &Path) -> Option<ConfigState> {
    let content = fs::read_to_string(state_path).ok()?;
    let mut lines = content.lines();
    let path = PathBuf::from(lines.next()?.trim());
    let mtime = lines.next()?.trim().parse().ok()?;
    if path.as_os_str().is_empty() {
        return None;
    }
    // Backwards compatibility: default to Explicit if not present
    // Also accept old names (PathGlob, Triggered) for backwards compatibility
    let source_type = lines
        .next()
        .and_then(|line| match line.trim() {
            "Explicit" => Some(ConfigSourceType::Explicit),
            "TriggerPath" | "PathGlob" => Some(ConfigSourceType::TriggerPath),
            "TriggerFile" | "Triggered" => Some(ConfigSourceType::TriggerFile),
            _ => None,
        })
        .unwrap_or(ConfigSourceType::Explicit);
    Some(ConfigState {
        path,
        mtime,
        source_type,
    })
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
    write_last_config_state_to(&state_file_path(), state);
}

/// Write the current config state to a specific file path.
/// Pass None to clear the state (when leaving a termtint project).
fn write_last_config_state_to(state_path: &Path, state: Option<&ConfigState>) {
    // Ensure parent directory exists
    if let Some(parent) = state_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    match state {
        Some(s) => {
            let source_type_str = match s.source_type {
                ConfigSourceType::Explicit => "Explicit",
                ConfigSourceType::TriggerPath => "TriggerPath",
                ConfigSourceType::TriggerFile => "TriggerFile",
            };
            let content = format!(
                "{}\n{}\n{}",
                s.path.to_string_lossy(),
                s.mtime,
                source_type_str
            );
            let _ = fs::write(state_path, content.as_bytes());
        }
        None => {
            let _ = fs::remove_file(state_path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_config_state_equality() {
        let state1 = ConfigState {
            path: PathBuf::from("/test/path"),
            mtime: 12345,
            source_type: ConfigSourceType::Explicit,
        };

        let state2 = ConfigState {
            path: PathBuf::from("/test/path"),
            mtime: 12345,
            source_type: ConfigSourceType::Explicit,
        };

        assert_eq!(state1, state2);
    }

    #[test]
    fn test_config_state_different_source_type() {
        let state1 = ConfigState {
            path: PathBuf::from("/test/path"),
            mtime: 12345,
            source_type: ConfigSourceType::Explicit,
        };

        let state2 = ConfigState {
            path: PathBuf::from("/test/path"),
            mtime: 12345,
            source_type: ConfigSourceType::TriggerFile,
        };

        assert_ne!(state1, state2);
    }

    #[test]
    fn test_write_and_read_state_explicit() {
        let temp = TempDir::new().unwrap();
        let state_path = state_file_path_for_home(temp.path());

        let state = ConfigState {
            path: PathBuf::from("/test/config/.termtint"),
            mtime: 67890,
            source_type: ConfigSourceType::Explicit,
        };

        write_last_config_state_to(&state_path, Some(&state));
        let read_state = read_last_config_state_from(&state_path);

        assert_eq!(read_state, Some(state));
    }

    #[test]
    fn test_write_and_read_state_triggered() {
        let temp = TempDir::new().unwrap();
        let state_path = state_file_path_for_home(temp.path());

        let state = ConfigState {
            path: PathBuf::from("/test/project"),
            mtime: 12345,
            source_type: ConfigSourceType::TriggerFile,
        };

        write_last_config_state_to(&state_path, Some(&state));
        let read_state = read_last_config_state_from(&state_path);

        assert_eq!(read_state, Some(state));
    }

    #[test]
    fn test_write_and_clear_state() {
        let temp = TempDir::new().unwrap();
        let state_path = state_file_path_for_home(temp.path());

        let state = ConfigState {
            path: PathBuf::from("/test/path"),
            mtime: 12345,
            source_type: ConfigSourceType::Explicit,
        };

        write_last_config_state_to(&state_path, Some(&state));
        assert!(read_last_config_state_from(&state_path).is_some());

        write_last_config_state_to(&state_path, None);
        assert_eq!(read_last_config_state_from(&state_path), None);
    }

    #[test]
    fn test_read_nonexistent_state() {
        let temp = TempDir::new().unwrap();
        let state_path = state_file_path_for_home(temp.path());

        let result = read_last_config_state_from(&state_path);
        assert_eq!(result, None);
    }

    #[test]
    fn test_backwards_compatibility_missing_source_type() {
        let temp = TempDir::new().unwrap();
        let state_path = state_file_path_for_home(temp.path());

        // Create parent directory and write old format (without source_type)
        fs::create_dir_all(state_path.parent().unwrap()).unwrap();
        fs::write(&state_path, "/test/path\n12345").unwrap();

        let state = read_last_config_state_from(&state_path).unwrap();

        // Should default to Explicit
        assert_eq!(state.path, PathBuf::from("/test/path"));
        assert_eq!(state.mtime, 12345);
        assert_eq!(state.source_type, ConfigSourceType::Explicit);
    }

    #[test]
    fn test_read_malformed_state() {
        let temp = TempDir::new().unwrap();
        let state_path = state_file_path_for_home(temp.path());

        // Create parent directory and write malformed content
        fs::create_dir_all(state_path.parent().unwrap()).unwrap();
        fs::write(&state_path, "invalid").unwrap();

        let result = read_last_config_state_from(&state_path);
        assert_eq!(result, None);
    }

    #[test]
    fn test_read_state_empty_path() {
        let temp = TempDir::new().unwrap();
        let state_path = state_file_path_for_home(temp.path());

        // Create parent directory and write state with empty path
        fs::create_dir_all(state_path.parent().unwrap()).unwrap();
        fs::write(&state_path, "\n12345\nExplicit").unwrap();

        let result = read_last_config_state_from(&state_path);
        assert_eq!(result, None);
    }

    #[test]
    fn test_get_file_mtime_exists() {
        let temp = TempDir::new().unwrap();
        let test_file = temp.path().join("test.txt");
        fs::write(&test_file, "content").unwrap();

        let mtime = get_file_mtime(&test_file);
        assert!(mtime.is_some());
        assert!(mtime.unwrap() > 0);
    }

    #[test]
    fn test_get_file_mtime_nonexistent() {
        let temp = TempDir::new().unwrap();
        let test_file = temp.path().join("nonexistent.txt");

        let mtime = get_file_mtime(&test_file);
        assert_eq!(mtime, None);
    }

    #[test]
    fn test_state_file_path_for_home() {
        let temp = TempDir::new().unwrap();

        let path = state_file_path_for_home(temp.path());
        assert_eq!(
            path,
            temp.path()
                .join(".cache")
                .join("termtint")
                .join("last_config")
        );
    }

    #[test]
    fn test_cleanup_stale_sessions() {
        let temp = TempDir::new().unwrap();
        let sessions_dir = sessions_dir_for_home(temp.path());

        // Create fresh session (should not be deleted)
        let fresh_session = sessions_dir.join("session1");
        fs::create_dir_all(&fresh_session).unwrap();
        fs::write(fresh_session.join("last_config"), "test").unwrap();

        // Create stale session (should be deleted)
        let stale_session = sessions_dir.join("session2");
        fs::create_dir_all(&stale_session).unwrap();
        let stale_file = stale_session.join("last_config");
        fs::write(&stale_file, "test").unwrap();

        // Set mtime to > 24 hours ago
        let old_time = SystemTime::now() - Duration::from_secs(25 * 60 * 60);
        let file_time = filetime::FileTime::from_system_time(old_time);
        filetime::set_file_mtime(&stale_file, file_time).unwrap();

        // Verify the mtime was actually set
        let metadata = fs::metadata(&stale_file).unwrap();
        let modified = metadata.modified().unwrap();
        let age = SystemTime::now().duration_since(modified).unwrap();
        assert!(
            age > Duration::from_secs(24 * 60 * 60),
            "File should be older than 24 hours"
        );

        cleanup_stale_sessions_in(&sessions_dir);

        // Fresh session should still exist
        assert!(
            fresh_session.exists(),
            "Fresh session should not be deleted"
        );

        // Stale session should be deleted
        assert!(!stale_session.exists(), "Stale session should be deleted");
    }

    #[test]
    fn test_cleanup_stale_sessions_no_sessions_dir() {
        let temp = TempDir::new().unwrap();
        let sessions_dir = sessions_dir_for_home(temp.path());

        // Should not panic when sessions dir doesn't exist
        cleanup_stale_sessions_in(&sessions_dir);
    }

    #[test]
    fn test_write_and_read_state_trigger_path() {
        let temp = TempDir::new().unwrap();
        let state_path = state_file_path_for_home(temp.path());

        let state = ConfigState {
            path: PathBuf::from("/test/project"),
            mtime: 0,
            source_type: ConfigSourceType::TriggerPath,
        };

        write_last_config_state_to(&state_path, Some(&state));

        let read_state = read_last_config_state_from(&state_path);

        assert_eq!(read_state, Some(state));
    }

    #[test]
    fn test_config_state_different_source_type_trigger_path() {
        let state1 = ConfigState {
            path: PathBuf::from("/test/path"),
            mtime: 12345,
            source_type: ConfigSourceType::TriggerPath,
        };

        let state2 = ConfigState {
            path: PathBuf::from("/test/path"),
            mtime: 12345,
            source_type: ConfigSourceType::TriggerFile,
        };

        assert_ne!(state1, state2);
    }
}
