use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RGB {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RGB {
    /// Create a darkened version of this color by the given percentage.
    /// A percent of 15 means the result will be 15% of the original brightness.
    pub fn darken(&self, percent: u8) -> RGB {
        RGB {
            r: (self.r as u16 * percent as u16 / 100) as u8,
            g: (self.g as u16 * percent as u16 / 100) as u8,
            b: (self.b as u16 * percent as u16 / 100) as u8,
        }
    }
}

/// Parse a hex color string like "#ff5500" into an RGB struct.
pub fn parse_hex(s: &str) -> Result<RGB, String> {
    let s = s.trim();
    let hex = s.strip_prefix('#').unwrap_or(s);

    if hex.len() != 6 {
        return Err(format!("Invalid hex color: expected 6 hex digits, got '{}'", s));
    }

    let r = u8::from_str_radix(&hex[0..2], 16)
        .map_err(|_| format!("Invalid hex color: '{}'", s))?;
    let g = u8::from_str_radix(&hex[2..4], 16)
        .map_err(|_| format!("Invalid hex color: '{}'", s))?;
    let b = u8::from_str_radix(&hex[4..6], 16)
        .map_err(|_| format!("Invalid hex color: '{}'", s))?;

    Ok(RGB { r, g, b })
}

#[derive(Debug, Clone, PartialEq)]
pub struct ColorConfig {
    pub tab: RGB,
    pub background: RGB,
}

#[derive(Debug, PartialEq)]
enum ConfigFormat {
    Hex,
    Toml,
    Auto,
}

/// Detect the format of a config file based on its content.
fn detect_format(content: &str) -> ConfigFormat {
    let trimmed = content.trim();
    if trimmed.starts_with('#') || trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        ConfigFormat::Hex
    } else if trimmed == "auto" {
        ConfigFormat::Auto
    } else {
        ConfigFormat::Toml
    }
}

/// Parse a simple hex color file. Derives background at 15% brightness.
fn parse_simple_hex(content: &str) -> Result<ColorConfig, String> {
    let tab = parse_hex(content)?;
    let background = tab.darken(15);
    Ok(ColorConfig { tab, background })
}

/// Parse a TOML config file.
fn parse_toml(content: &str) -> Result<ColorConfig, String> {
    let table: toml::Table = content
        .parse()
        .map_err(|e| format!("Failed to parse TOML: {}", e))?;

    let tab_str = table
        .get("tab")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'tab' key in TOML config")?;

    let tab = parse_hex(tab_str)?;

    let background = if let Some(bg_str) = table.get("background").and_then(|v| v.as_str()) {
        parse_hex(bg_str)?
    } else {
        tab.darken(15)
    };

    Ok(ColorConfig { tab, background })
}

/// Generate a deterministic color from the config file path.
fn parse_auto(path: &Path) -> ColorConfig {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let mut hasher = DefaultHasher::new();
    canonical.hash(&mut hasher);
    let hash = hasher.finish();

    // Use hash bytes to generate a vibrant color
    let r = ((hash >> 0) & 0xFF) as u8;
    let g = ((hash >> 8) & 0xFF) as u8;
    let b = ((hash >> 16) & 0xFF) as u8;

    // Ensure minimum brightness for visibility
    let tab = RGB {
        r: r.max(64),
        g: g.max(64),
        b: b.max(64),
    };
    let background = tab.darken(15);

    ColorConfig { tab, background }
}

/// Parse a config file at the given path.
pub fn parse_config(path: &Path) -> Result<ColorConfig, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;

    match detect_format(&content) {
        ConfigFormat::Hex => parse_simple_hex(&content),
        ConfigFormat::Toml => parse_toml(&content),
        ConfigFormat::Auto => Ok(parse_auto(path)),
    }
}

/// Find the nearest `.termtint` config file by walking up from start_dir.
/// Returns the path to the config file if found, None otherwise.
pub fn find_config(start_dir: &Path) -> Option<PathBuf> {
    let mut current = start_dir.to_path_buf();

    loop {
        let config_path = current.join(".termtint");
        if config_path.exists() {
            return Some(config_path);
        }

        if !current.pop() {
            // Reached root, no config found
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use tempfile::TempDir;

    #[test]
    fn test_config_in_current_dir() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join(".termtint");
        File::create(&config_path).unwrap();

        let result = find_config(temp.path());
        assert_eq!(result, Some(config_path));
    }

    #[test]
    fn test_config_in_parent_dir() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join(".termtint");
        File::create(&config_path).unwrap();

        let child_dir = temp.path().join("child");
        fs::create_dir(&child_dir).unwrap();

        let result = find_config(&child_dir);
        assert_eq!(result, Some(config_path));
    }

    #[test]
    fn test_no_config_found() {
        let temp = TempDir::new().unwrap();
        let child_dir = temp.path().join("child");
        fs::create_dir(&child_dir).unwrap();

        let result = find_config(&child_dir);
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_hex_with_hash() {
        let rgb = parse_hex("#ff5500").unwrap();
        assert_eq!(rgb, RGB { r: 255, g: 85, b: 0 });
    }

    #[test]
    fn test_parse_hex_without_hash() {
        let rgb = parse_hex("00ff00").unwrap();
        assert_eq!(rgb, RGB { r: 0, g: 255, b: 0 });
    }

    #[test]
    fn test_parse_hex_with_whitespace() {
        let rgb = parse_hex("  #aabbcc  ").unwrap();
        assert_eq!(rgb, RGB { r: 170, g: 187, b: 204 });
    }

    #[test]
    fn test_parse_hex_invalid_length() {
        let result = parse_hex("#fff");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_hex_invalid_chars() {
        let result = parse_hex("#gggggg");
        assert!(result.is_err());
    }

    #[test]
    fn test_rgb_darken() {
        let rgb = RGB { r: 100, g: 200, b: 50 };
        let darkened = rgb.darken(50);
        assert_eq!(darkened, RGB { r: 50, g: 100, b: 25 });
    }

    #[test]
    fn test_rgb_darken_15_percent() {
        let rgb = RGB { r: 255, g: 85, b: 0 };
        let darkened = rgb.darken(15);
        assert_eq!(darkened, RGB { r: 38, g: 12, b: 0 });
    }

    #[test]
    fn test_detect_format_hex_with_hash() {
        assert_eq!(detect_format("#ff5500"), ConfigFormat::Hex);
    }

    #[test]
    fn test_detect_format_hex_without_hash() {
        assert_eq!(detect_format("ff5500"), ConfigFormat::Hex);
    }

    #[test]
    fn test_detect_format_auto() {
        assert_eq!(detect_format("auto"), ConfigFormat::Auto);
        assert_eq!(detect_format("  auto  "), ConfigFormat::Auto);
    }

    #[test]
    fn test_detect_format_toml() {
        assert_eq!(detect_format("tab = \"#ff5500\""), ConfigFormat::Toml);
    }

    #[test]
    fn test_parse_simple_hex_config() {
        let config = parse_simple_hex("#ff5500").unwrap();
        assert_eq!(config.tab, RGB { r: 255, g: 85, b: 0 });
        assert_eq!(config.background, RGB { r: 38, g: 12, b: 0 });
    }

    #[test]
    fn test_parse_toml_with_tab_only() {
        let config = parse_toml("tab = \"#00ff00\"").unwrap();
        assert_eq!(config.tab, RGB { r: 0, g: 255, b: 0 });
        assert_eq!(config.background, RGB { r: 0, g: 38, b: 0 });
    }

    #[test]
    fn test_parse_toml_with_background() {
        let config = parse_toml("tab = \"#00ff00\"\nbackground = \"#001100\"").unwrap();
        assert_eq!(config.tab, RGB { r: 0, g: 255, b: 0 });
        assert_eq!(config.background, RGB { r: 0, g: 17, b: 0 });
    }

    #[test]
    fn test_parse_toml_missing_tab() {
        let result = parse_toml("background = \"#001100\"");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_auto_deterministic() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join(".termtint");
        fs::write(&config_path, "auto").unwrap();

        let config1 = parse_auto(&config_path);
        let config2 = parse_auto(&config_path);
        assert_eq!(config1, config2);
    }

    #[test]
    fn test_parse_config_hex_file() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join(".termtint");
        fs::write(&config_path, "#ff5500").unwrap();

        let config = parse_config(&config_path).unwrap();
        assert_eq!(config.tab, RGB { r: 255, g: 85, b: 0 });
    }

    #[test]
    fn test_parse_config_toml_file() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join(".termtint");
        fs::write(&config_path, "tab = \"#00ff00\"").unwrap();

        let config = parse_config(&config_path).unwrap();
        assert_eq!(config.tab, RGB { r: 0, g: 255, b: 0 });
    }

    #[test]
    fn test_parse_config_auto_file() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join(".termtint");
        fs::write(&config_path, "auto").unwrap();

        let config = parse_config(&config_path).unwrap();
        // Auto generates colors, just verify it doesn't error
        assert!(config.tab.r >= 64);
    }
}
