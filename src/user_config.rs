use std::fs;
use std::path::PathBuf;

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
    pub trigger_files: Vec<String>,
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
            trigger_files: Vec::new(),
        }
    }
}

/// TOML structure for parsing the config file.
#[derive(Debug, serde::Deserialize)]
struct UserConfigToml {
    #[serde(default)]
    background_lightness: Option<f32>,
    #[serde(default)]
    trigger_files: Option<Vec<String>>,
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
    if let Some(files) = toml_config.trigger_files {
        config.trigger_files = files;
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

/// Generate a default config TOML string with all settings and helpful comments.
pub fn default_config_toml() -> String {
    let defaults = UserConfig::default();
    format!(
        r#"# termtint user configuration
# Location: ~/.config/termtint/config.toml

# Fixed lightness for darkened backgrounds (0.0 to 1.0)
background_lightness = {:.2}

# Files that trigger automatic color generation when found
# Examples: ["Cargo.toml", "package.json", "go.mod", "pyproject.toml"]
trigger_files = []

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
        assert!(toml.contains("trigger_files = []"));
        assert!(toml.contains("[auto]"));
        assert!(toml.contains("hue_min = 0.0"));
        assert!(toml.contains("hue_max = 360.0"));
        assert!(toml.contains("saturation_min = 0.7"));
        assert!(toml.contains("saturation_max = 0.9"));
        assert!(toml.contains("lightness = 0.55"));

        // Should contain helpful comments
        assert!(toml.contains("# termtint user configuration"));
        assert!(toml.contains("# Fixed lightness for darkened backgrounds"));
        assert!(toml.contains("# Auto color generation parameters"));

        // Should be valid TOML that can be parsed back
        let parsed: Result<UserConfigToml, _> = toml::from_str(&toml);
        assert!(parsed.is_ok());

        // Verify parsed values match the expected defaults
        let parsed_toml = parsed.unwrap();
        let defaults = UserConfig::default();

        assert_eq!(parsed_toml.background_lightness.unwrap(), defaults.background_lightness);
        assert_eq!(parsed_toml.trigger_files.unwrap(), defaults.trigger_files);

        let auto = parsed_toml.auto.expect("auto section should be present");
        assert_eq!(auto.hue_min.unwrap(), defaults.hue_min);
        assert_eq!(auto.hue_max.unwrap(), defaults.hue_max);
        assert_eq!(auto.saturation_min.unwrap(), defaults.saturation_min);
        assert_eq!(auto.saturation_max.unwrap(), defaults.saturation_max);
        assert_eq!(auto.lightness.unwrap(), defaults.lightness);
    }
}
