use std::env;
use std::fs;

use crate::config;

/// Initialize a .termtint file in the current directory.
///
/// # Arguments
/// * `color` - Optional hex color for the tab (e.g., "#ff5500")
/// * `background` - Optional custom background color (hex)
/// * `force` - If true, overwrite existing .termtint file
///
/// # Returns
/// * `Ok(())` if successful
/// * `Err(String)` with error message if failed
pub fn cmd_init(
    color: Option<String>,
    background: Option<String>,
    force: bool,
) -> Result<(), String> {
    // 1. Get current directory
    let current_dir = env::current_dir()
        .map_err(|e| format!("Error getting current directory: {}", e))?;

    let config_path = current_dir.join(".termtint");

    // 2. Check if .termtint exists
    if config_path.exists() && !force {
        return Err(format!(
            "Error: .termtint already exists in this directory\nUse --force to overwrite"
        ));
    }

    // 3. Validate color arg if provided
    if let Some(ref color_str) = color {
        config::parse_color(color_str)
            .map_err(|e| format!("Invalid color: {}", e))?;
    }

    // 4. Validate background arg - requires color
    if background.is_some() && color.is_none() {
        return Err(
            "Error: --background requires an explicit tab color\nProvide a color argument or omit --background".to_string()
        );
    }

    // Validate background hex if provided
    if let Some(ref bg_str) = background {
        config::parse_color(bg_str)
            .map_err(|e| format!("Invalid background color: {}", e))?;
    }

    // 5. Generate file content based on arguments
    let content = match (color, background) {
        // No color: write "auto"
        (None, None) => "auto\n".to_string(),

        // Color only: write the hex color
        (Some(c), None) => {
            // Ensure color starts with #
            let normalized = if c.starts_with('#') {
                c
            } else {
                format!("#{}", c)
            };
            format!("{}\n", normalized)
        }

        // Color + background: write TOML format
        (Some(c), Some(bg)) => {
            // Ensure colors start with #
            let normalized_color = if c.starts_with('#') {
                c
            } else {
                format!("#{}", c)
            };
            let normalized_bg = if bg.starts_with('#') {
                bg
            } else {
                format!("#{}", bg)
            };
            format!("tab = \"{}\"\nbackground = \"{}\"\n", normalized_color, normalized_bg)
        }

        // This case is already handled by validation above
        (None, Some(_)) => unreachable!(),
    };

    // 6. Write to .termtint file
    fs::write(&config_path, content)
        .map_err(|e| format!("Error writing .termtint file: {}", e))?;

    // 7. Print success message
    println!("Created .termtint in {}", current_dir.display());

    // 8. Return Ok
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::sync::Mutex;
    use tempfile::TempDir;

    // Mutex to ensure tests that change current directory run serially
    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_init_creates_auto_file() {
        let _lock = TEST_MUTEX.lock().unwrap();
        let temp = TempDir::new().unwrap();
        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(temp.path()).unwrap();

        let result = cmd_init(None, None, false);
        assert!(result.is_ok());

        let config_path = temp.path().join(".termtint");
        assert!(config_path.exists());
        let content = fs::read_to_string(&config_path).unwrap();
        assert_eq!(content, "auto\n");

        env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_init_creates_hex_file() {
        let _lock = TEST_MUTEX.lock().unwrap();
        let temp = TempDir::new().unwrap();
        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(temp.path()).unwrap();

        let result = cmd_init(Some("#ff5500".to_string()), None, false);
        assert!(result.is_ok());

        let config_path = temp.path().join(".termtint");
        assert!(config_path.exists());
        let content = fs::read_to_string(&config_path).unwrap();
        assert_eq!(content, "#ff5500\n");

        env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_init_creates_toml_file() {
        let _lock = TEST_MUTEX.lock().unwrap();
        let temp = TempDir::new().unwrap();
        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(temp.path()).unwrap();

        let result = cmd_init(
            Some("#00ff00".to_string()),
            Some("#001100".to_string()),
            false,
        );
        assert!(result.is_ok());

        let config_path = temp.path().join(".termtint");
        assert!(config_path.exists());
        let content = fs::read_to_string(&config_path).unwrap();
        assert_eq!(content, "tab = \"#00ff00\"\nbackground = \"#001100\"\n");

        env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_init_fails_when_file_exists() {
        let _lock = TEST_MUTEX.lock().unwrap();
        let temp = TempDir::new().unwrap();
        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(temp.path()).unwrap();

        // Create .termtint first
        let config_path = temp.path().join(".termtint");
        fs::write(&config_path, "auto\n").unwrap();

        // Try to init without force
        let result = cmd_init(None, None, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already exists"));

        env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_init_force_overwrites() {
        let _lock = TEST_MUTEX.lock().unwrap();
        let temp = TempDir::new().unwrap();
        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(temp.path()).unwrap();

        // Create .termtint first
        let config_path = temp.path().join(".termtint");
        fs::write(&config_path, "auto\n").unwrap();

        // Init with force should succeed
        let result = cmd_init(Some("#ff5500".to_string()), None, true);
        assert!(result.is_ok());

        // Verify content was overwritten
        let content = fs::read_to_string(&config_path).unwrap();
        assert_eq!(content, "#ff5500\n");

        env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_init_rejects_invalid_color() {
        let _lock = TEST_MUTEX.lock().unwrap();
        let temp = TempDir::new().unwrap();
        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(temp.path()).unwrap();

        let result = cmd_init(Some("notacolor".to_string()), None, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid color"));

        env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_init_rejects_background_without_color() {
        let _lock = TEST_MUTEX.lock().unwrap();
        let temp = TempDir::new().unwrap();
        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(temp.path()).unwrap();

        let result = cmd_init(None, Some("#001100".to_string()), false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("--background requires an explicit tab color"));

        env::set_current_dir(original_dir).unwrap();
    }
}
