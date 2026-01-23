use std::env;
use std::fs;

use rand::Rng;

use crate::config;
use crate::iterm;
use crate::user_config::UserConfig;

/// Render a die with the given value (1-6) using Unicode box-drawing characters.
/// The die is rendered with the background color as the die face and the tab color for the dots.
fn render_die(value: u8, tab_color: &config::RGB, bg_color: &config::RGB) -> String {
    let bg = format!("\x1b[48;2;{};{};{}m", bg_color.r, bg_color.g, bg_color.b);
    let fg = format!("\x1b[38;2;{};{};{}m", tab_color.r, tab_color.g, tab_color.b);
    let reset = "\x1b[0m";

    // Define dot positions for each face (using ● for dots)
    let dots = match value {
        1 => vec![
            "       ",
            "   ●   ",
            "       ",
        ],
        2 => vec![
            " ●     ",
            "       ",
            "     ● ",
        ],
        3 => vec![
            " ●     ",
            "   ●   ",
            "     ● ",
        ],
        4 => vec![
            " ●   ● ",
            "       ",
            " ●   ● ",
        ],
        5 => vec![
            " ●   ● ",
            "   ●   ",
            " ●   ● ",
        ],
        6 => vec![
            " ●   ● ",
            " ●   ● ",
            " ●   ● ",
        ],
        _ => vec![
            "       ",
            "   ?   ",
            "       ",
        ],
    };

    let mut result = String::new();

    // Top border
    result.push_str(&format!("{}{} ┌───────┐ {}\n", reset, bg, reset));

    // Three rows of dots
    for dot_row in dots {
        result.push_str(&format!("{}{} │{}{}{}{}│ {}\n",
            reset, bg, fg, dot_row, reset, bg, reset));
    }

    // Bottom border
    result.push_str(&format!("{}{} └───────┘ {}\n", reset, bg, reset));

    result
}

/// Re-roll the color in an existing .termtint file with a new random color.
///
/// # Arguments
/// * `force` - If true, create .termtint if it doesn't exist
/// * `verbose` - If true, print directory path
/// * `user_config` - User configuration for color generation
///
/// # Returns
/// * `Ok(())` if successful
/// * `Err(String)` with error message if failed
pub fn cmd_reroll(force: bool, verbose: bool, user_config: &UserConfig) -> Result<(), String> {
    // 1. Get current directory
    let current_dir = env::current_dir()
        .map_err(|e| format!("Error getting current directory: {}", e))?;

    let config_path = current_dir.join(".termtint");

    // 2. Check if .termtint exists
    if !config_path.exists() && !force {
        return Err(
            "Error: .termtint does not exist in this directory\nUse --force to create a new one"
                .to_string(),
        );
    }

    // 3. Generate random color
    let rgb = config::generate_random_color(user_config);

    // 4. Format as hex string (RGB has Display trait that outputs #rrggbb)
    let hex_color = format!("{}\n", rgb);

    // 5. Write hex color + newline to .termtint
    fs::write(&config_path, &hex_color)
        .map_err(|e| format!("Error writing .termtint file: {}", e))?;

    // 6. Print success message (directory only with verbose)
    if verbose {
        println!("Re-rolled .termtint in {}\n", current_dir.display());
    }

    // 6a. Display dice with color info on the right
    let mut rng = rand::thread_rng();
    let die_value = rng.gen_range(1..=6);
    if let Ok(color_config) = config::parse_config(&config_path, user_config) {
        let die_output = render_die(die_value, &color_config.tab, &color_config.background);
        let lines: Vec<&str> = die_output.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            if i == 1 {
                // First dot row - show tab color
                println!(
                    "{}   Tab: {} {}",
                    line,
                    color_config.tab.format_as(user_config.color_format),
                    color_config.tab.as_color_block()
                );
            } else if i == 2 {
                // Second dot row - show background color
                println!(
                    "{}   Background: {} {}",
                    line,
                    color_config.background.format_as(user_config.color_format),
                    color_config.background.as_color_block()
                );
            } else {
                println!("{}", line);
            }
        }
    }

    // 7. Apply colors immediately
    if let Ok(color_config) = config::parse_config(&config_path, user_config) {
        iterm::apply_colors(&color_config);
    }

    Ok(())
}

/// Initialize a .termtint file in the current directory.
///
/// # Arguments
/// * `color` - Optional hex color for the tab (e.g., "#ff5500")
/// * `background` - Optional custom background color (hex)
/// * `force` - If true, overwrite existing .termtint file
/// * `user_config` - User configuration for color generation
///
/// # Returns
/// * `Ok(())` if successful
/// * `Err(String)` with error message if failed
pub fn cmd_init(
    color: Option<String>,
    background: Option<String>,
    force: bool,
    user_config: &UserConfig,
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
            // Parse color to RGB and use Display trait to format as hex
            let rgb = config::parse_color(&c)
                .map_err(|e| format!("Invalid color: {}", e))?;
            format!("{}\n", rgb)
        }

        // Color + background: write TOML format
        (Some(c), Some(bg)) => {
            // Parse colors to RGB and use Display trait to format as hex
            let rgb_color = config::parse_color(&c)
                .map_err(|e| format!("Invalid color: {}", e))?;
            let rgb_bg = config::parse_color(&bg)
                .map_err(|e| format!("Invalid background color: {}", e))?;
            format!("tab = \"{}\"\nbackground = \"{}\"\n", rgb_color, rgb_bg)
        }

        // This case is already handled by validation above
        (None, Some(_)) => unreachable!(),
    };

    // 6. Write to .termtint file
    fs::write(&config_path, content)
        .map_err(|e| format!("Error writing .termtint file: {}", e))?;

    // 7. Print success message
    println!("Created .termtint in {}", current_dir.display());

    // 8. Apply colors immediately
    if let Ok(color_config) = config::parse_config(&config_path, user_config) {
        iterm::apply_colors(&color_config);
    }

    // 9. Return Ok
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

        let user_config = UserConfig::default();
        let result = cmd_init(None, None, false, &user_config);
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

        let user_config = UserConfig::default();
        let result = cmd_init(Some("#ff5500".to_string()), None, false, &user_config);
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

        let user_config = UserConfig::default();
        let result = cmd_init(
            Some("#00ff00".to_string()),
            Some("#001100".to_string()),
            false,
            &user_config,
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
        let user_config = UserConfig::default();
        let result = cmd_init(None, None, false, &user_config);
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
        let user_config = UserConfig::default();
        let result = cmd_init(Some("#ff5500".to_string()), None, true, &user_config);
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

        let user_config = UserConfig::default();
        let result = cmd_init(Some("notacolor".to_string()), None, false, &user_config);
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

        let user_config = UserConfig::default();
        let result = cmd_init(None, Some("#001100".to_string()), false, &user_config);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("--background requires an explicit tab color"));

        env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_reroll_creates_hex_file() {
        let _lock = TEST_MUTEX.lock().unwrap();
        let temp = TempDir::new().unwrap();
        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(temp.path()).unwrap();

        // Create initial .termtint file
        let config_path = temp.path().join(".termtint");
        fs::write(&config_path, "#ff5500\n").unwrap();

        let user_config = UserConfig::default();
        let result = cmd_reroll(false, false, &user_config);
        assert!(result.is_ok());

        // Verify file exists and contains a valid hex color
        assert!(config_path.exists());
        let content = fs::read_to_string(&config_path).unwrap();
        assert!(content.starts_with('#'));
        assert_eq!(content.len(), 8); // #rrggbb\n

        // Verify the color changed from the original
        assert_ne!(content, "#ff5500\n");

        env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_reroll_fails_when_file_does_not_exist() {
        let _lock = TEST_MUTEX.lock().unwrap();
        let temp = TempDir::new().unwrap();
        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(temp.path()).unwrap();

        let user_config = UserConfig::default();
        let result = cmd_reroll(false, false, &user_config);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));

        env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_reroll_force_creates_file() {
        let _lock = TEST_MUTEX.lock().unwrap();
        let temp = TempDir::new().unwrap();
        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(temp.path()).unwrap();

        // No .termtint file exists
        let config_path = temp.path().join(".termtint");
        assert!(!config_path.exists());

        let user_config = UserConfig::default();
        let result = cmd_reroll(true, false, &user_config);
        assert!(result.is_ok());

        // Verify file was created with a valid hex color
        assert!(config_path.exists());
        let content = fs::read_to_string(&config_path).unwrap();
        assert!(content.starts_with('#'));
        assert_eq!(content.len(), 8); // #rrggbb\n

        env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_reroll_produces_different_colors() {
        let _lock = TEST_MUTEX.lock().unwrap();
        let temp = TempDir::new().unwrap();
        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(temp.path()).unwrap();

        let config_path = temp.path().join(".termtint");
        let user_config = UserConfig::default();

        // Generate multiple colors by re-rolling
        let mut colors = Vec::new();
        for _ in 0..5 {
            cmd_reroll(true, false, &user_config).unwrap();
            let content = fs::read_to_string(&config_path).unwrap();
            colors.push(content.trim().to_string());
        }

        // At least some should be different (highly unlikely all 5 are the same)
        let first_color = &colors[0];
        let has_different_color = colors.iter().any(|c| c != first_color);
        assert!(
            has_different_color,
            "Should generate different random colors, but all were {}",
            first_color
        );

        env::set_current_dir(original_dir).unwrap();
    }
}
