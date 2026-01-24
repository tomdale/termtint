use std::fs;
use std::path::{Path, PathBuf};

/// Color format for displaying colors.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ColorFormat {
    #[default]
    Hex,
    Hsl,
    Rgb,
}

/// Get the config file path for a given home directory.
fn config_file_path_for_home(home: &Path) -> PathBuf {
    home.join(".config").join("termtint").join("config.toml")
}

/// Get the path to the user config file.
pub fn config_file_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    config_file_path_for_home(Path::new(&home))
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
    pub trigger_paths: Vec<String>,
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
            background_lightness: 0.18,
            background_saturation: 1.0,
            trigger_files: Vec::new(),
            trigger_paths: Vec::new(),
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
    trigger_paths: Option<Vec<String>>,
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
    load_user_config_from(&config_file_path())
}

/// Load user configuration from a specific file path.
/// Returns default config if file doesn't exist or can't be parsed.
fn load_user_config_from(config_path: &Path) -> UserConfig {
    // Return default if file doesn't exist
    let Ok(content) = fs::read_to_string(config_path) else {
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
    if let Some(paths) = toml_config.trigger_paths {
        config.trigger_paths = paths;
    }
    if let Some(format_str) = toml_config.color_format {
        config.color_format = match format_str.to_lowercase().as_str() {
            "hsl" => ColorFormat::Hsl,
            "rgb" => ColorFormat::Rgb,
            "hex" => ColorFormat::Hex,
            _ => {
                eprintln!(
                    "termtint: warning: invalid color_format '{}', using hex",
                    format_str
                );
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
    let content =
        toml::to_string_pretty(&table).map_err(|e| format!("Error serializing config: {}", e))?;
    fs::write(&config_path, content).map_err(|e| format!("Error writing config file: {}", e))?;

    Ok(())
}

/// Save trigger paths to the user config, preserving other settings.
pub fn save_trigger_paths(trigger_paths: &[String]) -> Result<(), String> {
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

    // Update trigger_paths
    let paths_array: Vec<toml::Value> = trigger_paths
        .iter()
        .map(|s| toml::Value::String(s.clone()))
        .collect();
    table.insert("trigger_paths".to_string(), toml::Value::Array(paths_array));

    // Write back
    let content =
        toml::to_string_pretty(&table).map_err(|e| format!("Error serializing config: {}", e))?;
    fs::write(&config_path, content).map_err(|e| format!("Error writing config file: {}", e))?;

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

# Path globs that trigger automatic color generation
# Directories matching these patterns are treated as having 'auto' in .termtint
# Supports ~ for home directory. Example: ["~/Code/*", "~/Projects/*"]
trigger_paths = []

# Color format for display: "hex", "hsl", or "rgb"
color_format = "hex"

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

/// Template for a config field, used for upgrading existing configs.
struct FieldTemplate {
    /// Field name (e.g., "background_saturation")
    name: &'static str,
    /// Template lines including comment and commented-out default value
    template: &'static str,
    /// Whether this field belongs in the [auto] section
    in_auto_section: bool,
}

/// All known config fields with their templates.
/// These are used to add missing fields to existing config files.
const FIELD_TEMPLATES: &[FieldTemplate] = &[
    // Top-level fields (in order they should appear)
    FieldTemplate {
        name: "background_lightness",
        template: "# Fixed lightness for darkened backgrounds (0.0 to 1.0)\n# background_lightness = 0.18",
        in_auto_section: false,
    },
    FieldTemplate {
        name: "background_saturation",
        template: "# Saturation multiplier for backgrounds (0.0 to 1.0)\n# 1.0 = preserve original saturation, 0.0 = grayscale\n# background_saturation = 1.00",
        in_auto_section: false,
    },
    FieldTemplate {
        name: "trigger_files",
        template: "# Files that trigger automatic color generation when found\n# Examples: [\"Cargo.toml\", \"package.json\", \"go.mod\", \"pyproject.toml\"]\n# trigger_files = []",
        in_auto_section: false,
    },
    FieldTemplate {
        name: "trigger_paths",
        template: "# Path globs that trigger automatic color generation\n# Directories matching these patterns are treated as having 'auto' in .termtint\n# Supports ~ for home directory. Example: [\"~/Code/*\", \"~/Projects/*\"]\n# trigger_paths = []",
        in_auto_section: false,
    },
    FieldTemplate {
        name: "color_format",
        template: "# Color format for display: \"hex\", \"hsl\", or \"rgb\"\n# color_format = \"hex\"",
        in_auto_section: false,
    },
    // [auto] section fields
    FieldTemplate {
        name: "hue_min",
        template: "# Hue range in degrees (0.0 to 360.0)\n# hue_min = 0.0",
        in_auto_section: true,
    },
    FieldTemplate {
        name: "hue_max",
        template: "# hue_max = 360.0",
        in_auto_section: true,
    },
    FieldTemplate {
        name: "saturation_min",
        template: "# Saturation range (0.0 to 1.0)\n# saturation_min = 0.7",
        in_auto_section: true,
    },
    FieldTemplate {
        name: "saturation_max",
        template: "# saturation_max = 0.9",
        in_auto_section: true,
    },
    FieldTemplate {
        name: "lightness",
        template: "# Lightness for generated tab colors (0.0 to 1.0)\n# lightness = 0.55",
        in_auto_section: true,
    },
];

/// Detect which config fields are present in the content.
/// Returns (set of field names, whether [auto] section exists, line number of [auto] header).
fn detect_present_fields(
    content: &str,
) -> (std::collections::HashSet<String>, bool, Option<usize>) {
    use std::collections::HashSet;

    let known_fields: HashSet<&str> = FIELD_TEMPLATES.iter().map(|f| f.name).collect();
    let mut found_fields = HashSet::new();
    let mut has_auto_section = false;
    let mut auto_section_line = None;

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        // Check for [auto] section header
        if trimmed == "[auto]" {
            has_auto_section = true;
            auto_section_line = Some(line_num);
            continue;
        }

        // Check for field assignment (active or commented)
        // Handles both "field = value" and "# field = value"
        let check_line = if let Some(stripped) = trimmed.strip_prefix('#') {
            stripped.trim_start()
        } else {
            trimmed
        };

        if let Some(eq_pos) = check_line.find('=') {
            let field_name = check_line[..eq_pos].trim();
            if known_fields.contains(field_name) {
                found_fields.insert(field_name.to_string());
            }
        }
    }

    (found_fields, has_auto_section, auto_section_line)
}

/// Upgrade an existing config file by adding missing fields as commented-out defaults.
/// Preserves all existing content and only adds fields that are completely absent.
pub fn upgrade_config(content: &str) -> String {
    let (found_fields, _has_auto_section, auto_section_line) = detect_present_fields(content);

    // Find missing fields
    let missing_top_level: Vec<&FieldTemplate> = FIELD_TEMPLATES
        .iter()
        .filter(|f| !f.in_auto_section && !found_fields.contains(f.name))
        .collect();

    let missing_auto: Vec<&FieldTemplate> = FIELD_TEMPLATES
        .iter()
        .filter(|f| f.in_auto_section && !found_fields.contains(f.name))
        .collect();

    // If nothing is missing, return original content
    if missing_top_level.is_empty() && missing_auto.is_empty() {
        return content.to_string();
    }

    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

    // Insert top-level fields before [auto] section or at end
    if !missing_top_level.is_empty() {
        let insert_point = auto_section_line.unwrap_or(lines.len());

        let mut to_insert: Vec<String> = Vec::new();

        // Add blank line separator if needed
        if insert_point > 0
            && !lines
                .get(insert_point.saturating_sub(1))
                .map(|s| s.trim().is_empty())
                .unwrap_or(true)
        {
            to_insert.push(String::new());
        }

        for (i, spec) in missing_top_level.iter().enumerate() {
            if i > 0 {
                to_insert.push(String::new());
            }
            to_insert.extend(spec.template.lines().map(|s| s.to_string()));
        }

        // Add trailing blank line if inserting before [auto]
        if auto_section_line.is_some() {
            to_insert.push(String::new());
        }

        // Insert the lines
        for (i, line) in to_insert.into_iter().enumerate() {
            lines.insert(insert_point + i, line);
        }
    }

    // Insert [auto] section fields
    if !missing_auto.is_empty() {
        // Recalculate auto section position after possible top-level insertions
        let (_, has_auto_now, _auto_line_now) = detect_present_fields(&lines.join("\n"));

        if !has_auto_now {
            // Need to create [auto] section
            if !lines.last().map(|s| s.trim().is_empty()).unwrap_or(true) {
                lines.push(String::new());
            }
            lines.push("# Auto color generation parameters".to_string());
            lines.push("[auto]".to_string());
        }

        // Find end of [auto] section (end of file since it's the last section)
        let auto_end = lines.len();

        let mut to_insert: Vec<String> = Vec::new();
        for (i, spec) in missing_auto.iter().enumerate() {
            if i > 0 {
                to_insert.push(String::new());
            }
            to_insert.extend(spec.template.lines().map(|s| s.to_string()));
        }

        for line in to_insert {
            lines.insert(auto_end, line);
        }
    }

    // Ensure file ends with newline
    let result = lines.join("\n");
    if result.ends_with('\n') {
        result
    } else {
        result + "\n"
    }
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
        assert_eq!(config.background_lightness, 0.18);
        assert_eq!(config.background_saturation, 1.0);
        assert!(config.trigger_files.is_empty());
        assert!(config.trigger_paths.is_empty());
    }

    #[test]
    fn test_load_missing_config() {
        let temp = TempDir::new().unwrap();
        let config_path = config_file_path_for_home(temp.path());

        let config = load_user_config_from(&config_path);

        // Should return defaults
        assert_eq!(config.hue_min, 0.0);
        assert_eq!(config.background_lightness, 0.18);
        assert!(config.trigger_files.is_empty());
    }

    #[test]
    fn test_load_empty_config() {
        let temp = TempDir::new().unwrap();
        let config_path = config_file_path_for_home(temp.path());
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        fs::write(&config_path, "").unwrap();

        let config = load_user_config_from(&config_path);

        // Should return defaults
        assert_eq!(config.background_lightness, 0.18);
        assert!(config.trigger_files.is_empty());
    }

    #[test]
    fn test_load_partial_config() {
        let temp = TempDir::new().unwrap();
        let config_path = config_file_path_for_home(temp.path());
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();

        let content = r#"
background_lightness = 0.15
"#;
        fs::write(&config_path, content).unwrap();

        let config = load_user_config_from(&config_path);

        // Should override only specified values
        assert_eq!(config.background_lightness, 0.15);
        assert_eq!(config.hue_min, 0.0); // default
        assert!(config.trigger_files.is_empty()); // default
    }

    #[test]
    fn test_load_full_config() {
        let temp = TempDir::new().unwrap();
        let config_path = config_file_path_for_home(temp.path());
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();

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

        let config = load_user_config_from(&config_path);

        assert_eq!(config.background_lightness, 0.12);
        assert_eq!(
            config.trigger_files,
            vec!["Cargo.toml", "package.json", "pyproject.toml"]
        );
        assert_eq!(config.hue_min, 10.0);
        assert_eq!(config.hue_max, 350.0);
        assert_eq!(config.saturation_min, 0.6);
        assert_eq!(config.saturation_max, 0.8);
        assert_eq!(config.lightness, 0.50);
    }

    #[test]
    fn test_load_auto_section_only() {
        let temp = TempDir::new().unwrap();
        let config_path = config_file_path_for_home(temp.path());
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();

        let content = r#"
[auto]
hue_min = 120.0
hue_max = 240.0
"#;
        fs::write(&config_path, content).unwrap();

        let config = load_user_config_from(&config_path);

        assert_eq!(config.hue_min, 120.0);
        assert_eq!(config.hue_max, 240.0);
        assert_eq!(config.background_lightness, 0.18); // default
        assert_eq!(config.saturation_min, 0.7); // default
    }

    #[test]
    fn test_load_malformed_config() {
        let temp = TempDir::new().unwrap();
        let config_path = config_file_path_for_home(temp.path());
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();

        // Invalid TOML
        fs::write(&config_path, "not valid toml {[}]").unwrap();

        let config = load_user_config_from(&config_path);

        // Should return defaults on parse error
        assert_eq!(config.background_lightness, 0.18);
        assert!(config.trigger_files.is_empty());
    }

    #[test]
    fn test_trigger_files_empty_array() {
        let temp = TempDir::new().unwrap();
        let config_path = config_file_path_for_home(temp.path());
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();

        let content = r#"
trigger_files = []
"#;
        fs::write(&config_path, content).unwrap();

        let config = load_user_config_from(&config_path);

        assert!(config.trigger_files.is_empty());
    }

    #[test]
    fn test_config_file_path_for_home() {
        let temp = TempDir::new().unwrap();

        let path = config_file_path_for_home(temp.path());
        assert_eq!(
            path,
            temp.path()
                .join(".config")
                .join("termtint")
                .join("config.toml")
        );
    }

    #[test]
    fn test_default_config_toml() {
        let toml = default_config_toml();

        // Should contain all expected sections
        assert!(toml.contains("background_lightness = 0.18"));
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

        assert_eq!(
            parsed_toml.background_lightness.unwrap(),
            defaults.background_lightness
        );
        assert_eq!(
            parsed_toml.background_saturation.unwrap(),
            defaults.background_saturation
        );
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
        let config_path = config_file_path_for_home(temp.path());
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();

        let content = r#"
color_format = "hex"
"#;
        fs::write(&config_path, content).unwrap();

        let config = load_user_config_from(&config_path);

        assert!(matches!(config.color_format, ColorFormat::Hex));
    }

    #[test]
    fn test_load_config_with_hsl_format() {
        let temp = TempDir::new().unwrap();
        let config_path = config_file_path_for_home(temp.path());
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();

        let content = r#"
color_format = "hsl"
"#;
        fs::write(&config_path, content).unwrap();

        let config = load_user_config_from(&config_path);

        assert!(matches!(config.color_format, ColorFormat::Hsl));
    }

    #[test]
    fn test_load_config_with_rgb_format() {
        let temp = TempDir::new().unwrap();
        let config_path = config_file_path_for_home(temp.path());
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();

        let content = r#"
color_format = "rgb"
"#;
        fs::write(&config_path, content).unwrap();

        let config = load_user_config_from(&config_path);

        assert!(matches!(config.color_format, ColorFormat::Rgb));
    }

    #[test]
    fn test_load_config_with_invalid_format() {
        let temp = TempDir::new().unwrap();
        let config_path = config_file_path_for_home(temp.path());
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();

        let content = r#"
color_format = "invalid"
"#;
        fs::write(&config_path, content).unwrap();

        let config = load_user_config_from(&config_path);

        // Should fall back to hex (default) on invalid format
        assert!(matches!(config.color_format, ColorFormat::Hex));
    }

    #[test]
    fn test_load_config_format_case_insensitive() {
        let temp = TempDir::new().unwrap();
        let config_path = config_file_path_for_home(temp.path());
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();

        let content = r#"
color_format = "HSL"
"#;
        fs::write(&config_path, content).unwrap();

        let config = load_user_config_from(&config_path);

        // Should handle uppercase
        assert!(matches!(config.color_format, ColorFormat::Hsl));
    }

    #[test]
    fn test_load_config_with_background_saturation() {
        let temp = TempDir::new().unwrap();
        let config_path = config_file_path_for_home(temp.path());
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();

        let content = r#"
background_saturation = 0.5
"#;
        fs::write(&config_path, content).unwrap();

        let config = load_user_config_from(&config_path);

        assert_eq!(config.background_saturation, 0.5);
        // Other values should be defaults
        assert_eq!(config.background_lightness, 0.18);
    }

    #[test]
    fn test_load_config_background_saturation_clamped() {
        let temp = TempDir::new().unwrap();
        let config_path = config_file_path_for_home(temp.path());
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();

        // Test value above 1.0 is clamped
        let content = r#"
background_saturation = 2.0
"#;
        fs::write(&config_path, content).unwrap();

        let config = load_user_config_from(&config_path);

        assert_eq!(config.background_saturation, 1.0);
    }

    #[test]
    fn test_load_config_with_trigger_paths() {
        let temp = TempDir::new().unwrap();
        let config_path = config_file_path_for_home(temp.path());
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();

        let content = r#"
trigger_paths = ["~/Code/*", "~/Projects/*"]
"#;
        fs::write(&config_path, content).unwrap();

        let config = load_user_config_from(&config_path);

        assert_eq!(config.trigger_paths, vec!["~/Code/*", "~/Projects/*"]);
    }

    #[test]
    fn test_default_config_has_empty_trigger_paths() {
        let config = UserConfig::default();
        assert!(config.trigger_paths.is_empty());
    }

    // Tests for upgrade_config functionality

    #[test]
    fn test_upgrade_adds_missing_field() {
        // Config with only background_lightness - should add background_saturation
        let content = r#"background_lightness = 0.15
"#;
        let upgraded = upgrade_config(content);

        // Should contain the original value
        assert!(upgraded.contains("background_lightness = 0.15"));

        // Should add background_saturation as a comment
        assert!(upgraded.contains("# background_saturation = 1.00"));
    }

    #[test]
    fn test_upgrade_preserves_user_values() {
        let content = r#"background_lightness = 0.20
background_saturation = 0.5
"#;
        let upgraded = upgrade_config(content);

        // User values should be preserved exactly
        assert!(upgraded.contains("background_lightness = 0.20"));
        assert!(upgraded.contains("background_saturation = 0.5"));

        // Should not add a duplicate commented background_saturation
        assert!(!upgraded.contains("# background_saturation = 1.00"));
    }

    #[test]
    fn test_upgrade_no_duplicate_for_commented_field() {
        // User has already commented out background_saturation with custom value
        let content = r#"background_lightness = 0.15
# background_saturation = 0.3
"#;
        let upgraded = upgrade_config(content);

        // Should keep the user's commented value
        assert!(upgraded.contains("# background_saturation = 0.3"));

        // Should not add another commented default
        let count = upgraded.matches("background_saturation").count();
        assert_eq!(count, 1, "Should have exactly one background_saturation");
    }

    #[test]
    fn test_upgrade_creates_auto_section() {
        // Config without [auto] section
        let content = r#"background_lightness = 0.15
background_saturation = 0.8
trigger_files = []
trigger_paths = []
color_format = "hex"
"#;
        let upgraded = upgrade_config(content);

        // Should create [auto] section with all fields
        assert!(upgraded.contains("[auto]"));
        assert!(upgraded.contains("# hue_min = 0.0"));
        assert!(upgraded.contains("# hue_max = 360.0"));
        assert!(upgraded.contains("# saturation_min = 0.7"));
    }

    #[test]
    fn test_upgrade_adds_to_existing_auto_section() {
        // Config with partial [auto] section
        let content = r#"background_lightness = 0.15
background_saturation = 0.8
trigger_files = []
trigger_paths = []
color_format = "hex"

[auto]
hue_min = 30.0
hue_max = 60.0
"#;
        let upgraded = upgrade_config(content);

        // Should keep existing values
        assert!(upgraded.contains("hue_min = 30.0"));
        assert!(upgraded.contains("hue_max = 60.0"));

        // Should add missing auto fields
        assert!(upgraded.contains("# saturation_min = 0.7"));
        assert!(upgraded.contains("# saturation_max = 0.9"));
        assert!(upgraded.contains("# lightness = 0.55"));
    }

    #[test]
    fn test_upgrade_empty_file() {
        let content = "";
        let upgraded = upgrade_config(content);

        // Should add all fields as comments
        assert!(upgraded.contains("# background_lightness = 0.18"));
        assert!(upgraded.contains("# background_saturation = 1.00"));
        assert!(upgraded.contains("# trigger_files = []"));
        assert!(upgraded.contains("[auto]"));
        assert!(upgraded.contains("# hue_min = 0.0"));
    }

    #[test]
    fn test_upgrade_complete_file_unchanged() {
        // A config with all fields present (active)
        let content = r#"background_lightness = 0.18
background_saturation = 1.00
trigger_files = []
trigger_paths = []
color_format = "hex"

[auto]
hue_min = 0.0
hue_max = 360.0
saturation_min = 0.7
saturation_max = 0.9
lightness = 0.55
"#;
        let upgraded = upgrade_config(content);

        // Content should be essentially unchanged (just newline normalization)
        assert!(upgraded.contains("background_lightness = 0.18"));
        assert!(upgraded.contains("background_saturation = 1.00"));
        assert!(upgraded.contains("hue_min = 0.0"));

        // Should not have any commented defaults since all fields are present
        assert!(!upgraded.contains("# background_lightness"));
        assert!(!upgraded.contains("# hue_min"));
    }

    #[test]
    fn test_upgrade_with_user_comments() {
        // Config with user's own comments
        let content = r#"# My termtint config
background_lightness = 0.15

# I like this saturation
background_saturation = 0.6
"#;
        let upgraded = upgrade_config(content);

        // User comments should be preserved
        assert!(upgraded.contains("# My termtint config"));
        assert!(upgraded.contains("# I like this saturation"));

        // User values should be preserved
        assert!(upgraded.contains("background_lightness = 0.15"));
        assert!(upgraded.contains("background_saturation = 0.6"));
    }
}
