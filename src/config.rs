use csscolorparser;
use oklab::{oklab_to_srgb, srgb_to_oklab, Oklab, Rgb};
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
    /// Create a darkened version using perceptually uniform Oklab color space.
    /// target_lightness is the desired fraction of original lightness (0.0 to 1.0).
    pub fn darken(&self, target_lightness: f32) -> RGB {
        // Convert to Oklab (the crate handles sRGB u8 conversion)
        let srgb = Rgb {
            r: self.r,
            g: self.g,
            b: self.b,
        };

        // Convert to Oklab, reduce lightness, convert back
        let oklab = srgb_to_oklab(srgb);
        let darkened_oklab = Oklab {
            l: oklab.l * target_lightness,
            a: oklab.a,
            b: oklab.b,
        };
        let darkened_srgb = oklab_to_srgb(darkened_oklab);

        RGB {
            r: darkened_srgb.r,
            g: darkened_srgb.g,
            b: darkened_srgb.b,
        }
    }
}

/// Parse a color string in any supported format:
/// - 6-digit hex: "#ff5500" or "ff5500"
/// - 3-digit hex: "#f50"
/// - RGB function: "rgb(255, 85, 0)"
/// - HSL function: "hsl(20, 100%, 50%)"
/// - Named colors: "red", "tomato", etc.
pub fn parse_color(s: &str) -> Result<RGB, String> {
    let s = s.trim();

    // Handle bare 6-digit hex without # prefix for backwards compatibility
    let normalized = if s.chars().all(|c| c.is_ascii_hexdigit()) && s.len() == 6 {
        format!("#{}", s)
    } else {
        s.to_string()
    };

    let color = csscolorparser::parse(&normalized)
        .map_err(|e| format!("Invalid color '{}': {}", s, e))?;

    let [r, g, b, _a] = color.to_rgba8();
    Ok(RGB { r, g, b })
}

#[derive(Debug, Clone, PartialEq)]
pub struct ColorConfig {
    pub tab: RGB,
    pub background: RGB,
}

#[derive(Debug, PartialEq)]
enum ConfigFormat {
    SimpleColor,
    Toml,
    Auto,
}

/// Detect the format of a config file based on its content.
fn detect_format(content: &str) -> ConfigFormat {
    let trimmed = content.trim();
    if trimmed == "auto" {
        ConfigFormat::Auto
    } else if trimmed.contains('=') {
        ConfigFormat::Toml
    } else {
        ConfigFormat::SimpleColor
    }
}

/// Parse a simple color file. Derives background at 15% brightness.
fn parse_simple_color(content: &str) -> Result<ColorConfig, String> {
    let tab = parse_color(content)?;
    let background = tab.darken(0.15);
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

    let tab = parse_color(tab_str)?;

    let background = if let Some(bg_str) = table.get("background").and_then(|v| v.as_str()) {
        parse_color(bg_str)?
    } else {
        tab.darken(0.15)
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
    let background = tab.darken(0.15);

    ColorConfig { tab, background }
}

/// Parse a config file at the given path.
pub fn parse_config(path: &Path) -> Result<ColorConfig, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;

    match detect_format(&content) {
        ConfigFormat::SimpleColor => parse_simple_color(&content),
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
    fn test_parse_color_hex_with_hash() {
        let rgb = parse_color("#ff5500").unwrap();
        assert_eq!(rgb, RGB { r: 255, g: 85, b: 0 });
    }

    #[test]
    fn test_parse_color_hex_without_hash() {
        let rgb = parse_color("00ff00").unwrap();
        assert_eq!(rgb, RGB { r: 0, g: 255, b: 0 });
    }

    #[test]
    fn test_parse_color_hex_with_whitespace() {
        let rgb = parse_color("  #aabbcc  ").unwrap();
        assert_eq!(rgb, RGB { r: 170, g: 187, b: 204 });
    }

    #[test]
    fn test_parse_color_hex_3digit() {
        let rgb = parse_color("#f50").unwrap();
        assert_eq!(rgb, RGB { r: 255, g: 85, b: 0 });
    }

    #[test]
    fn test_parse_color_hex_3digit_abc() {
        let rgb = parse_color("#abc").unwrap();
        assert_eq!(rgb, RGB { r: 170, g: 187, b: 204 });
    }

    #[test]
    fn test_parse_color_rgb_function() {
        let rgb = parse_color("rgb(255, 85, 0)").unwrap();
        assert_eq!(rgb, RGB { r: 255, g: 85, b: 0 });
    }

    #[test]
    fn test_parse_color_hsl_function() {
        let rgb = parse_color("hsl(20, 100%, 50%)").unwrap();
        assert_eq!(rgb, RGB { r: 255, g: 85, b: 0 });
    }

    #[test]
    fn test_parse_color_hsl_red() {
        let rgb = parse_color("hsl(0, 100%, 50%)").unwrap();
        assert_eq!(rgb, RGB { r: 255, g: 0, b: 0 });
    }

    #[test]
    fn test_parse_color_hsl_green() {
        let rgb = parse_color("hsl(120, 100%, 50%)").unwrap();
        assert_eq!(rgb, RGB { r: 0, g: 255, b: 0 });
    }

    #[test]
    fn test_parse_color_named_red() {
        let rgb = parse_color("red").unwrap();
        assert_eq!(rgb, RGB { r: 255, g: 0, b: 0 });
    }

    #[test]
    fn test_parse_color_named_tomato() {
        let rgb = parse_color("tomato").unwrap();
        assert_eq!(rgb, RGB { r: 255, g: 99, b: 71 });
    }

    #[test]
    fn test_parse_color_invalid() {
        let result = parse_color("#gggggg");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_color_invalid_notacolor() {
        let result = parse_color("notacolor");
        assert!(result.is_err());
    }

    #[test]
    fn test_rgb_darken() {
        let rgb = RGB { r: 100, g: 200, b: 50 };
        let darkened = rgb.darken(0.50);
        // With Oklab perceptual darkening, 50% lightness preserves hue
        assert_eq!(darkened, RGB { r: 0, g: 84, b: 0 });
    }

    #[test]
    fn test_rgb_darken_15_percent() {
        let rgb = RGB { r: 255, g: 85, b: 0 };
        let darkened = rgb.darken(0.15);
        // With Oklab perceptual darkening, 15% lightness preserves hue better
        assert_eq!(darkened, RGB { r: 48, g: 0, b: 0 });
    }

    #[test]
    fn test_detect_format_simple_color_with_hash() {
        assert_eq!(detect_format("#ff5500"), ConfigFormat::SimpleColor);
    }

    #[test]
    fn test_detect_format_simple_color_without_hash() {
        assert_eq!(detect_format("ff5500"), ConfigFormat::SimpleColor);
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
    fn test_detect_format_named_color() {
        assert_eq!(detect_format("red"), ConfigFormat::SimpleColor);
        assert_eq!(detect_format("tomato"), ConfigFormat::SimpleColor);
    }

    #[test]
    fn test_parse_simple_color_config() {
        let config = parse_simple_color("#ff5500").unwrap();
        assert_eq!(config.tab, RGB { r: 255, g: 85, b: 0 });
        assert_eq!(config.background, RGB { r: 48, g: 0, b: 0 });
    }

    #[test]
    fn test_parse_toml_with_tab_only() {
        let config = parse_toml("tab = \"#00ff00\"").unwrap();
        assert_eq!(config.tab, RGB { r: 0, g: 255, b: 0 });
        assert_eq!(config.background, RGB { r: 0, g: 21, b: 0 });
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
    fn test_parse_toml_with_hsl() {
        let config = parse_toml("tab = \"hsl(0, 100%, 50%)\"").unwrap();
        assert_eq!(config.tab, RGB { r: 255, g: 0, b: 0 });
        assert_eq!(config.background, RGB { r: 54, g: 0, b: 0 });
    }

    #[test]
    fn test_parse_toml_with_named_color() {
        let config = parse_toml("tab = \"tomato\"").unwrap();
        assert_eq!(config.tab, RGB { r: 255, g: 99, b: 71 });
        assert_eq!(config.background, RGB { r: 46, g: 0, b: 0 });
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
