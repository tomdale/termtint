use std::fs;
use std::path::PathBuf;

/// Color format for displaying colors.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColorFormat {
    Hex,
    Hsl,
    Rgb,
}

impl Default for ColorFormat {
    fn default() -> Self {
        ColorFormat::Hex
    }
}

/// Get the path to the user config file.
pub fn config_file_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".config")
        .join("termtint")
        .join("config.toml")
}

/// User configuration for termtint behavior.
#[derive(Debug, Clone)]
pub struct UserConfig {
    // Auto color generation parameters
    pub hue_min: f32,
    pub hue_max: f32,
    pub saturation_min: f32,
    pub saturation_max: f32,
    pub lightness: f32,
    pub background_lightness: f32,
    pub background_saturation: f32,
    pub trigger_files: Vec<String>,
    pub color_format: ColorFormat,
}

impl Default for UserConfig {
    fn default() -> Self {
        UserConfig {
            hue_min: 0.0,
            hue_max: 360.0,
            saturation_min: 0.7,
            saturation_max: 0.9,
            lightness: 0.55,
            background_lightness: 0.10,
            background_saturation: 1.0,
            trigger_files: Vec::new(),
            color_format: ColorFormat::default(),
        }
    }
}

/// TOML structure for parsing the config file.
#[derive(Debug, serde::Deserialize)]
struct UserConfigToml {
    #[serde(default)]
    background_lightness: Option<f32>,
    #[serde(default)]
    background_saturation: Option<f32>,
    #[serde(default)]
    trigger_files: Option<Vec<String>>,
    #[serde(default)]
    color_format: Option<String>,
    #[serde(default)]
    auto: Option<AutoConfig>,
}

#[derive(Debug, serde::Deserialize)]
struct AutoConfig {
    #[serde(default)]
    hue_min: Option<f32>,
    #[serde(default)]
    hue_max: Option<f32>,
    #[serde(default)]
    saturation_min: Option<f32>,
    #[serde(default)]
    saturation_max: Option<f32>,
    #[serde(default)]
    lightness: Option<f32>,
}

/// Load user configuration from ~/.config/termtint/config.toml.
/// Returns default config if file doesn't exist or can't be parsed.
pub fn load_user_config() -> UserConfig {
    let config_path = config_file_path();

    // Return default if file doesn't exist
    let Ok(content) = fs::read_to_string(&config_path) else {
        return UserConfig::default();
    };

    // Parse TOML
    let Ok(toml_config): Result<UserConfigToml, _> = toml::from_str(&content) else {
        eprintln!("termtint: warning: failed to parse user config, using defaults");
        return UserConfig::default();
    };

    // Start with defaults
    let mut config = UserConfig::default();

    // Apply top-level overrides
    if let Some(lightness) = toml_config.background_lightness {
        config.background_lightness = lightness;
    }
    if let Some(saturation) = toml_config.background_saturation {
        config.background_saturation = saturation.clamp(0.0, 1.0);
    }
    if let Some(files) = toml_config.trigger_files {
        config.trigger_files = files;
    }
    if let Some(format_str) = toml_config.color_format {
        config.color_format = match format_str.to_lowercase().as_str() {
            "hsl" => ColorFormat::Hsl,
            "rgb" => ColorFormat::Rgb,
            "hex" => ColorFormat::Hex,
            _ => {
                eprintln!("termtint: warning: invalid color_format '{}', using hex", format_str);
                ColorFormat::Hex
            }
        };
    }

    // Apply auto section overrides
    if let Some(auto) = toml_config.auto {
        if let Some(v) = auto.hue_min {
            config.hue_min = v;
        }
        if let Some(v) = auto.hue_max {
            config.hue_max = v;
        }
        if let Some(v) = auto.saturation_min {
            config.saturation_min = v;
        }
        if let Some(v) = auto.saturation_max {
            config.saturation_max = v;
        }
        if let Some(v) = auto.lightness {
            config.lightness = v;
        }
    }

    config
}

/// Save trigger files to the user config, preserving other settings.
pub fn save_trigger_files(trigger_files: &[String]) -> Result<(), String> {
    let config_path = config_file_path();

    // Create parent directories if needed
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Error creating config directory: {}", e))?;
    }

    // Read existing config or start fresh
    let mut table: toml::Table = if config_path.exists() {
        let content = fs::read_to_string(&config_path)
            .map_err(|e| format!("Error reading config file: {}", e))?;
        toml::from_str(&content).unwrap_or_default()
    } else {
        toml::Table::new()
    };

    // Update trigger_files
    let files_array: Vec<toml::Value> = trigger_files
        .iter()
        .map(|s| toml::Value::String(s.clone()))
        .collect();
    table.insert("trigger_files".to_string(), toml::Value::Array(files_array));

    // Write back
    let content = toml::to_string_pretty(&table)
        .map_err(|e| format!("Error serializing config: {}", e))?;
    fs::write(&config_path, content)
        .map_err(|e| format!("Error writing config file: {}", e))?;

    Ok(())
}

/// Generate a default config TOML string with all settings and helpful comments.
pub fn default_config_toml() -> String {
    let defaults = UserConfig::default();
    format!(
        r#"# termtint user configuration
# Location: ~/.config/termtint/config.toml

# Fixed lightness for darkened backgrounds (0.0 to 1.0)
background_lightness = {:.2}

# Saturation multiplier for backgrounds (0.0 to 1.0)
# 1.0 = preserve original saturation, 0.0 = grayscale
background_saturation = {:.2}

# Files that trigger automatic color generation when found
# Examples: ["Cargo.toml", "package.json", "go.mod", "pyproject.toml"]
trigger_files = []

# Color format for display: "hex", "hsl", or "rgb"
# color_format = "hex"

# Auto color generation parameters
[auto]
# Hue range in degrees (0.0 to 360.0)
hue_min = {:.1}
hue_max = {:.1}

# Saturation range (0.0 to 1.0)
saturation_min = {:.1}
saturation_max = {:.1}

# Lightness for generated tab colors (0.0 to 1.0)
lightness = {:.2}
"#,
        defaults.background_lightness,
        defaults.background_saturation,
        defaults.hue_min,
        defaults.hue_max,
        defaults.saturation_min,
        defaults.saturation_max,
        defaults.lightness
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = UserConfig::default();
        assert_eq!(config.hue_min, 0.0);
        assert_eq!(config.hue_max, 360.0);
        assert_eq!(config.saturation_min, 0.7);
        assert_eq!(config.saturation_max, 0.9);
        assert_eq!(config.lightness, 0.55);
        assert_eq!(config.background_lightness, 0.10);
        assert_eq!(config.background_saturation, 1.0);
        assert!(config.trigger_files.is_empty());
    }

    #[test]
    fn test_load_missing_config() {
        // Set HOME to non-existent directory to ensure no config file exists
        let temp = TempDir::new().unwrap();
        std::env::set_var("HOME", temp.path());

        let config = load_user_config();

        // Should return defaults
        assert_eq!(config.hue_min, 0.0);
        assert_eq!(config.background_lightness, 0.10);
        assert!(config.trigger_files.is_empty());
    }

    #[test]
    fn test_load_empty_config() {
        let temp = TempDir::new().unwrap();
        let config_dir = temp.path().join(".config").join("termtint");
        fs::create_dir_all(&config_dir).unwrap();
        let config_path = config_dir.join("config.toml");
        fs::write(&config_path, "").unwrap();

        std::env::set_var("HOME", temp.path());

        let config = load_user_config();

        // Should return defaults
        assert_eq!(config.background_lightness, 0.10);
        assert!(config.trigger_files.is_empty());
    }

    #[test]
    fn test_load_partial_config() {
        let temp = TempDir::new().unwrap();
        let config_dir = temp.path().join(".config").join("termtint");
        fs::create_dir_all(&config_dir).unwrap();
        let config_path = config_dir.join("config.toml");

        let content = r#"
background_lightness = 0.15
"#;
        fs::write(&config_path, content).unwrap();

        std::env::set_var("HOME", temp.path());

        let config = load_user_config();

        // Should override only specified values
        assert_eq!(config.background_lightness, 0.15);
        assert_eq!(config.hue_min, 0.0); // default
        assert!(config.trigger_files.is_empty()); // default
    }

    #[test]
    fn test_load_full_config() {
        let temp = TempDir::new().unwrap();
        let config_dir = temp.path().join(".config").join("termtint");
        fs::create_dir_all(&config_dir).unwrap();
        let config_path = config_dir.join("config.toml");

        let content = r#"
background_lightness = 0.12
trigger_files = ["Cargo.toml", "package.json", "pyproject.toml"]

[auto]
hue_min = 10.0
hue_max = 350.0
saturation_min = 0.6
saturation_max = 0.8
lightness = 0.50
"#;
        fs::write(&config_path, content).unwrap();

        std::env::set_var("HOME", temp.path());

        let config = load_user_config();

        assert_eq!(config.background_lightness, 0.12);
        assert_eq!(config.trigger_files, vec!["Cargo.toml", "package.json", "pyproject.toml"]);
        assert_eq!(config.hue_min, 10.0);
        assert_eq!(config.hue_max, 350.0);
        assert_eq!(config.saturation_min, 0.6);
        assert_eq!(config.saturation_max, 0.8);
        assert_eq!(config.lightness, 0.50);
    }

    #[test]
    fn test_load_auto_section_only() {
        let temp = TempDir::new().unwrap();
        let config_dir = temp.path().join(".config").join("termtint");
        fs::create_dir_all(&config_dir).unwrap();
        let config_path = config_dir.join("config.toml");

        let content = r#"
[auto]
hue_min = 120.0
hue_max = 240.0
"#;
        fs::write(&config_path, content).unwrap();

        std::env::set_var("HOME", temp.path());

        let config = load_user_config();

        assert_eq!(config.hue_min, 120.0);
        assert_eq!(config.hue_max, 240.0);
        assert_eq!(config.background_lightness, 0.10); // default
        assert_eq!(config.saturation_min, 0.7); // default
    }

    #[test]
    fn test_load_malformed_config() {
        let temp = TempDir::new().unwrap();
        let config_dir = temp.path().join(".config").join("termtint");
        fs::create_dir_all(&config_dir).unwrap();
        let config_path = config_dir.join("config.toml");

        // Invalid TOML
        fs::write(&config_path, "not valid toml {[}]").unwrap();

        std::env::set_var("HOME", temp.path());

        let config = load_user_config();

        // Should return defaults on parse error
        assert_eq!(config.background_lightness, 0.10);
        assert!(config.trigger_files.is_empty());
    }

    #[test]
    fn test_trigger_files_empty_array() {
        let temp = TempDir::new().unwrap();
        let config_dir = temp.path().join(".config").join("termtint");
        fs::create_dir_all(&config_dir).unwrap();
        let config_path = config_dir.join("config.toml");

        let content = r#"
trigger_files = []
"#;
        fs::write(&config_path, content).unwrap();

        std::env::set_var("HOME", temp.path());

        let config = load_user_config();

        assert!(config.trigger_files.is_empty());
    }

    #[test]
    fn test_config_file_path() {
        let temp = TempDir::new().unwrap();
        std::env::set_var("HOME", temp.path());

        let path = config_file_path();
        assert_eq!(
            path,
            temp.path().join(".config").join("termtint").join("config.toml")
        );
    }

    #[test]
    fn test_default_config_toml() {
        let toml = default_config_toml();

        // Should contain all expected sections
        assert!(toml.contains("background_lightness = 0.10"));
        assert!(toml.contains("background_saturation = 1.00"));
        assert!(toml.contains("trigger_files = []"));
        assert!(toml.contains("[auto]"));
        assert!(toml.contains("hue_min = 0.0"));
        assert!(toml.contains("hue_max = 360.0"));
        assert!(toml.contains("saturation_min = 0.7"));
        assert!(toml.contains("saturation_max = 0.9"));
        assert!(toml.contains("lightness = 0.55"));
        assert!(toml.contains("color_format"));

        // Should contain helpful comments
        assert!(toml.contains("# termtint user configuration"));
        assert!(toml.contains("# Fixed lightness for darkened backgrounds"));
        assert!(toml.contains("# Saturation multiplier for backgrounds"));
        assert!(toml.contains("# Auto color generation parameters"));

        // Should be valid TOML that can be parsed back
        let parsed: Result<UserConfigToml, _> = toml::from_str(&toml);
        assert!(parsed.is_ok());

        // Verify parsed values match the expected defaults
        let parsed_toml = parsed.unwrap();
        let defaults = UserConfig::default();

        assert_eq!(parsed_toml.background_lightness.unwrap(), defaults.background_lightness);
        assert_eq!(parsed_toml.background_saturation.unwrap(), defaults.background_saturation);
        assert_eq!(parsed_toml.trigger_files.unwrap(), defaults.trigger_files);

        let auto = parsed_toml.auto.expect("auto section should be present");
        assert_eq!(auto.hue_min.unwrap(), defaults.hue_min);
        assert_eq!(auto.hue_max.unwrap(), defaults.hue_max);
        assert_eq!(auto.saturation_min.unwrap(), defaults.saturation_min);
        assert_eq!(auto.saturation_max.unwrap(), defaults.saturation_max);
        assert_eq!(auto.lightness.unwrap(), defaults.lightness);
    }

    #[test]
    fn test_load_config_with_hex_format() {
        let temp = TempDir::new().unwrap();
        let config_dir = temp.path().join(".config").join("termtint");
        fs::create_dir_all(&config_dir).unwrap();
        let config_path = config_dir.join("config.toml");

        let content = r#"
color_format = "hex"
"#;
        fs::write(&config_path, content).unwrap();

        std::env::set_var("HOME", temp.path());

        let config = load_user_config();

        assert!(matches!(config.color_format, ColorFormat::Hex));
    }

    #[test]
    fn test_load_config_with_hsl_format() {
        let temp = TempDir::new().unwrap();
        let config_dir = temp.path().join(".config").join("termtint");
        fs::create_dir_all(&config_dir).unwrap();
        let config_path = config_dir.join("config.toml");

        let content = r#"
color_format = "hsl"
"#;
        fs::write(&config_path, content).unwrap();

        std::env::set_var("HOME", temp.path());

        let config = load_user_config();

        assert!(matches!(config.color_format, ColorFormat::Hsl));
    }

    #[test]
    fn test_load_config_with_rgb_format() {
        let temp = TempDir::new().unwrap();
        let config_dir = temp.path().join(".config").join("termtint");
        fs::create_dir_all(&config_dir).unwrap();
        let config_path = config_dir.join("config.toml");

        let content = r#"
color_format = "rgb"
"#;
        fs::write(&config_path, content).unwrap();

        std::env::set_var("HOME", temp.path());

        let config = load_user_config();

        assert!(matches!(config.color_format, ColorFormat::Rgb));
    }

    #[test]
    fn test_load_config_with_invalid_format() {
        let temp = TempDir::new().unwrap();
        let config_dir = temp.path().join(".config").join("termtint");
        fs::create_dir_all(&config_dir).unwrap();
        let config_path = config_dir.join("config.toml");

        let content = r#"
color_format = "invalid"
"#;
        fs::write(&config_path, content).unwrap();

        std::env::set_var("HOME", temp.path());

        let config = load_user_config();

        // Should fall back to hex (default) on invalid format
        assert!(matches!(config.color_format, ColorFormat::Hex));
    }

    #[test]
    fn test_load_config_format_case_insensitive() {
        let temp = TempDir::new().unwrap();
        let config_dir = temp.path().join(".config").join("termtint");
        fs::create_dir_all(&config_dir).unwrap();
        let config_path = config_dir.join("config.toml");

        let content = r#"
color_format = "HSL"
"#;
        fs::write(&config_path, content).unwrap();

        std::env::set_var("HOME", temp.path());

        let config = load_user_config();

        // Should handle uppercase
        assert!(matches!(config.color_format, ColorFormat::Hsl));
    }

    #[test]
    fn test_load_config_with_background_saturation() {
        let temp = TempDir::new().unwrap();
        let config_dir = temp.path().join(".config").join("termtint");
        fs::create_dir_all(&config_dir).unwrap();
        let config_path = config_dir.join("config.toml");

        let content = r#"
background_saturation = 0.5
"#;
        fs::write(&config_path, content).unwrap();

        std::env::set_var("HOME", temp.path());

        let config = load_user_config();

        assert_eq!(config.background_saturation, 0.5);
        // Other values should be defaults
        assert_eq!(config.background_lightness, 0.10);
    }

    #[test]
    fn test_load_config_background_saturation_clamped() {
        let temp = TempDir::new().unwrap();
        let config_dir = temp.path().join(".config").join("termtint");
        fs::create_dir_all(&config_dir).unwrap();
        let config_path = config_dir.join("config.toml");

        // Test value above 1.0 is clamped
        let content = r#"
background_saturation = 2.0
"#;
        fs::write(&config_path, content).unwrap();

        std::env::set_var("HOME", temp.path());

        let config = load_user_config();

        assert_eq!(config.background_saturation, 1.0);
    }
}
