use crate::user_config::UserConfig;
use csscolorparser;
use oklab::{oklab_to_srgb, srgb_to_oklab, Oklab, Rgb};
use rand::Rng;
use std::collections::hash_map::DefaultHasher;
use std::fmt;
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
    /// Create a version with a fixed lightness using perceptually uniform Oklab color space.
    /// Preserves hue and chroma while setting the lightness to the target value.
    pub fn with_lightness(&self, target_lightness: f32) -> RGB {
        self.with_lightness_and_saturation(target_lightness, 1.0)
    }

    /// Create a version with adjusted lightness and saturation using Oklab color space.
    /// The saturation_factor scales the chroma (a, b components):
    /// - 1.0 = preserve original saturation
    /// - 0.0 = grayscale
    /// - 0.5 = 50% saturation
    pub fn with_lightness_and_saturation(&self, target_lightness: f32, saturation_factor: f32) -> RGB {
        // Convert to Oklab (the crate handles sRGB u8 conversion)
        let srgb = Rgb {
            r: self.r,
            g: self.g,
            b: self.b,
        };

        // Convert to Oklab, set lightness and scale chroma, convert back
        let oklab = srgb_to_oklab(srgb);
        let saturation_factor = saturation_factor.clamp(0.0, 1.0);
        let modified_oklab = Oklab {
            l: target_lightness.clamp(0.0, 1.0),
            a: oklab.a * saturation_factor,
            b: oklab.b * saturation_factor,
        };
        let modified_srgb = oklab_to_srgb(modified_oklab);

        RGB {
            r: modified_srgb.r,
            g: modified_srgb.g,
            b: modified_srgb.b,
        }
    }

    /// Format the color in the specified format.
    pub fn format_as(&self, format: crate::user_config::ColorFormat) -> String {
        use crate::user_config::ColorFormat;
        match format {
            ColorFormat::Hex => format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b),
            ColorFormat::Rgb => format!("rgb({}, {}, {})", self.r, self.g, self.b),
            ColorFormat::Hsl => {
                // Convert RGB to HSL using csscolorparser
                let hex = format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b);
                let color = csscolorparser::parse(&hex).unwrap();
                let [h, s, l, _a] = color.to_hsla();
                format!("hsl({:.0}, {:.0}%, {:.0}%)", h, s * 100.0, l * 100.0)
            }
        }
    }
}

impl fmt::Display for RGB {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

impl RGB {
    /// Format as a colored unicode block using ANSI true color escape sequences.
    pub fn as_color_block(&self) -> String {
        format!(
            "\x1b[48;2;{};{};{}m  \x1b[0m",
            self.r, self.g, self.b
        )
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

/// Represents the source of a color configuration.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigSource {
    /// Explicit .termtint file found
    Termtint(PathBuf),
    /// Directory with a trigger file (e.g., Cargo.toml, package.json)
    TriggerFile(String),
}

#[derive(Debug, PartialEq)]
pub enum ConfigFormat {
    SimpleColor,
    Toml,
    Auto,
}

/// Detect the format of a config file based on its content.
pub fn detect_format(content: &str) -> ConfigFormat {
    let trimmed = content.trim();
    if trimmed == "auto" {
        ConfigFormat::Auto
    } else if trimmed.contains('=') {
        ConfigFormat::Toml
    } else {
        ConfigFormat::SimpleColor
    }
}

/// Parse a simple color file. Derives background using configured lightness and saturation.
fn parse_simple_color(content: &str, user_config: &UserConfig) -> Result<ColorConfig, String> {
    let tab = parse_color(content)?;
    let background = tab.with_lightness_and_saturation(
        user_config.background_lightness,
        user_config.background_saturation,
    );
    Ok(ColorConfig { tab, background })
}

/// Parse a TOML config file.
fn parse_toml(content: &str, user_config: &UserConfig) -> Result<ColorConfig, String> {
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
        tab.with_lightness_and_saturation(
            user_config.background_lightness,
            user_config.background_saturation,
        )
    };

    Ok(ColorConfig { tab, background })
}

/// Generate a deterministic color from the config file path using user-configured parameters.
fn parse_auto(path: &Path, user_config: &UserConfig) -> ColorConfig {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let mut hasher = DefaultHasher::new();
    canonical.hash(&mut hasher);
    let hash = hasher.finish();

    // Use HSL color space for vibrant colors
    // Derive hue from hash within configured range
    let hue_range = user_config.hue_max - user_config.hue_min;
    let hue = user_config.hue_min + ((hash & 0xFFFF) as f32 / 0xFFFF as f32) * hue_range;

    // Use configured saturation range
    let saturation_range = user_config.saturation_max - user_config.saturation_min;
    let saturation = user_config.saturation_min + ((hash >> 16) & 0xFF) as f32 / 0xFF as f32 * saturation_range;

    // Use configured fixed lightness
    let lightness = user_config.lightness;

    // Create color using HSL and convert to RGB
    let color = csscolorparser::Color::from_hsla(hue, saturation, lightness, 1.0);
    let [r, g, b, _a] = color.to_rgba8();

    let tab = RGB { r, g, b };
    let background = tab.with_lightness_and_saturation(
        user_config.background_lightness,
        user_config.background_saturation,
    );

    ColorConfig { tab, background }
}

/// Generate a random color using user-configured parameters.
pub fn generate_random_color(user_config: &UserConfig) -> RGB {
    let mut rng = rand::thread_rng();
    let random_value = rng.gen::<u64>();

    // Use HSL color space for vibrant colors
    // Derive hue from random value within configured range
    let hue_range = user_config.hue_max - user_config.hue_min;
    let hue = user_config.hue_min + ((random_value & 0xFFFF) as f32 / 0xFFFF as f32) * hue_range;

    // Use configured saturation range
    let saturation_range = user_config.saturation_max - user_config.saturation_min;
    let saturation = user_config.saturation_min + ((random_value >> 16) & 0xFF) as f32 / 0xFF as f32 * saturation_range;

    // Use configured fixed lightness
    let lightness = user_config.lightness;

    // Create color using HSL and convert to RGB
    let color = csscolorparser::Color::from_hsla(hue, saturation, lightness, 1.0);
    let [r, g, b, _a] = color.to_rgba8();

    RGB { r, g, b }
}

/// Parse a config file at the given path.
pub fn parse_config(path: &Path, user_config: &UserConfig) -> Result<ColorConfig, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;

    match detect_format(&content) {
        ConfigFormat::SimpleColor => parse_simple_color(&content, user_config),
        ConfigFormat::Toml => parse_toml(&content, user_config),
        ConfigFormat::Auto => Ok(parse_auto(path, user_config)),
    }
}

/// Find a configuration source by walking up from start_dir.
/// First checks for explicit `.termtint` files (highest priority),
/// then checks for trigger files defined in user_config.
/// Returns ConfigSource describing where the config comes from, or None if nothing found.
pub fn find_config_source(start_dir: &Path, user_config: &UserConfig) -> Option<ConfigSource> {
    let mut current = start_dir.to_path_buf();

    loop {
        // First priority: check for explicit .termtint file
        let termtint_path = current.join(".termtint");
        if termtint_path.exists() {
            return Some(ConfigSource::Termtint(termtint_path));
        }

        // Second priority: check for any trigger files
        for trigger_file in &user_config.trigger_files {
            let trigger_path = current.join(trigger_file);
            if trigger_path.exists() {
                return Some(ConfigSource::TriggerFile(current.to_string_lossy().to_string()));
            }
        }

        if !current.pop() {
            // Reached root, no config found
            return None;
        }
    }
}

/// Parse a config from a ConfigSource.
/// For Termtint sources, reads and parses the .termtint file.
/// For TriggerFile sources, generates an auto color based on the directory path.
pub fn parse_config_source(source: &ConfigSource, user_config: &UserConfig) -> Result<ColorConfig, String> {
    match source {
        ConfigSource::Termtint(path) => parse_config(path, user_config),
        ConfigSource::TriggerFile(dir_path) => {
            // Generate auto color based on directory path
            let dir = PathBuf::from(dir_path);
            Ok(parse_auto(&dir, user_config))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use tempfile::TempDir;

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
    fn test_rgb_with_lightness() {
        let rgb = RGB { r: 100, g: 200, b: 50 };
        let darkened = rgb.with_lightness(0.10);
        // Setting Oklab lightness to 0.10 preserves hue
        assert_eq!(darkened, RGB { r: 0, g: 9, b: 0 });
    }

    #[test]
    fn test_rgb_with_lightness_different_value() {
        let rgb = RGB { r: 255, g: 85, b: 0 };
        let darkened = rgb.with_lightness(0.15);
        // Setting Oklab lightness to 0.15 preserves hue
        assert_eq!(darkened, RGB { r: 66, g: 0, b: 0 });
    }

    #[test]
    fn test_rgb_with_lightness_and_saturation_full_saturation() {
        let rgb = RGB { r: 255, g: 85, b: 0 };
        // Full saturation (1.0) should produce the same result as with_lightness
        let result = rgb.with_lightness_and_saturation(0.15, 1.0);
        let expected = rgb.with_lightness(0.15);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_rgb_with_lightness_and_saturation_zero_saturation() {
        let rgb = RGB { r: 255, g: 85, b: 0 };
        // Zero saturation should produce grayscale
        let result = rgb.with_lightness_and_saturation(0.15, 0.0);
        // Grayscale means r == g == b
        assert_eq!(result.r, result.g);
        assert_eq!(result.g, result.b);
    }

    #[test]
    fn test_rgb_with_lightness_and_saturation_half_saturation() {
        let rgb = RGB { r: 255, g: 85, b: 0 };
        // Half saturation should be somewhere between full and grayscale
        let full = rgb.with_lightness_and_saturation(0.15, 1.0);
        let half = rgb.with_lightness_and_saturation(0.15, 0.5);
        let zero = rgb.with_lightness_and_saturation(0.15, 0.0);

        // Half saturation result should have less color difference than full
        let full_diff = (full.r as i32 - full.g as i32).abs() + (full.g as i32 - full.b as i32).abs();
        let half_diff = (half.r as i32 - half.g as i32).abs() + (half.g as i32 - half.b as i32).abs();
        let zero_diff = (zero.r as i32 - zero.g as i32).abs() + (zero.g as i32 - zero.b as i32).abs();

        assert!(half_diff < full_diff, "Half saturation should have less chroma than full");
        assert!(half_diff > zero_diff, "Half saturation should have more chroma than zero");
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
        let user_config = UserConfig::default();
        let config = parse_simple_color("#ff5500", &user_config).unwrap();
        assert_eq!(config.tab, RGB { r: 255, g: 85, b: 0 });
        // Background uses fixed lightness (0.10 by default)
        assert_eq!(config.background, RGB { r: 48, g: 0, b: 0 });
    }

    #[test]
    fn test_parse_toml_with_tab_only() {
        let user_config = UserConfig::default();
        let config = parse_toml("tab = \"#00ff00\"", &user_config).unwrap();
        assert_eq!(config.tab, RGB { r: 0, g: 255, b: 0 });
        // Background uses fixed lightness (0.10 by default)
        assert_eq!(config.background, RGB { r: 0, g: 13, b: 0 });
    }

    #[test]
    fn test_parse_toml_with_background() {
        let user_config = UserConfig::default();
        let config = parse_toml("tab = \"#00ff00\"\nbackground = \"#001100\"", &user_config).unwrap();
        assert_eq!(config.tab, RGB { r: 0, g: 255, b: 0 });
        assert_eq!(config.background, RGB { r: 0, g: 17, b: 0 });
    }

    #[test]
    fn test_parse_toml_missing_tab() {
        let user_config = UserConfig::default();
        let result = parse_toml("background = \"#001100\"", &user_config);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_toml_with_hsl() {
        let user_config = UserConfig::default();
        let config = parse_toml("tab = \"hsl(0, 100%, 50%)\"", &user_config).unwrap();
        assert_eq!(config.tab, RGB { r: 255, g: 0, b: 0 });
        // Background uses fixed lightness (0.10 by default)
        assert_eq!(config.background, RGB { r: 56, g: 0, b: 0 });
    }

    #[test]
    fn test_parse_toml_with_named_color() {
        let user_config = UserConfig::default();
        let config = parse_toml("tab = \"tomato\"", &user_config).unwrap();
        assert_eq!(config.tab, RGB { r: 255, g: 99, b: 71 });
        // Background uses fixed lightness (0.10 by default)
        assert_eq!(config.background, RGB { r: 44, g: 0, b: 0 });
    }

    #[test]
    fn test_parse_auto_deterministic() {
        let user_config = UserConfig::default();
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join(".termtint");
        fs::write(&config_path, "auto").unwrap();

        let config1 = parse_auto(&config_path, &user_config);
        let config2 = parse_auto(&config_path, &user_config);
        assert_eq!(config1, config2);
    }

    #[test]
    fn test_parse_auto_produces_vibrant_colors() {
        // Test multiple different paths to ensure vibrancy constraints hold
        let user_config = UserConfig::default();
        let temp = TempDir::new().unwrap();

        let test_paths = vec![
            temp.path().join("project1").join(".termtint"),
            temp.path().join("project2").join(".termtint"),
            temp.path().join("project3").join(".termtint"),
            temp.path().join("deeply").join("nested").join("path").join(".termtint"),
        ];

        for path in test_paths {
            // Create parent directories
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&path, "auto").unwrap();

            let config = parse_auto(&path, &user_config);
            let rgb = config.tab;

            // Convert RGB back to HSL to verify constraints
            // Using csscolorparser to convert back
            let color_str = format!("#{:02x}{:02x}{:02x}", rgb.r, rgb.g, rgb.b);
            let color = csscolorparser::parse(&color_str).unwrap();
            let [_hue, saturation, lightness, _alpha] = color.to_hsla();

            // Verify saturation is >= 0.7 (high saturation for vibrant colors)
            assert!(
                saturation >= 0.7,
                "Path {:?} generated color {} with saturation {}, expected >= 0.7",
                path,
                color_str,
                saturation
            );

            // Verify saturation is <= 0.9 (upper bound from implementation)
            assert!(
                saturation <= 0.9,
                "Path {:?} generated color {} with saturation {}, expected <= 0.9",
                path,
                color_str,
                saturation
            );

            // Verify lightness is approximately the configured value (0.55 default)
            // Allow small tolerance for floating point/color space conversion
            let expected_lightness = user_config.lightness;
            assert!(
                (lightness - expected_lightness).abs() < 0.02,
                "Path {:?} generated color {} with lightness {}, expected ~{}",
                path,
                color_str,
                lightness,
                expected_lightness
            );
        }
    }

    #[test]
    fn test_parse_config_hex_file() {
        let user_config = UserConfig::default();
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join(".termtint");
        fs::write(&config_path, "#ff5500").unwrap();

        let config = parse_config(&config_path, &user_config).unwrap();
        assert_eq!(config.tab, RGB { r: 255, g: 85, b: 0 });
    }

    #[test]
    fn test_parse_config_toml_file() {
        let user_config = UserConfig::default();
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join(".termtint");
        fs::write(&config_path, "tab = \"#00ff00\"").unwrap();

        let config = parse_config(&config_path, &user_config).unwrap();
        assert_eq!(config.tab, RGB { r: 0, g: 255, b: 0 });
    }

    #[test]
    fn test_parse_config_auto_file() {
        let user_config = UserConfig::default();
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join(".termtint");
        fs::write(&config_path, "auto").unwrap();

        let config = parse_config(&config_path, &user_config).unwrap();
        // Auto generates vibrant colors using HSL, verify at least one channel is bright
        let max_channel = config.tab.r.max(config.tab.g).max(config.tab.b);
        assert!(max_channel >= 128, "Generated color should be vibrant with at least one bright channel");
    }

    #[test]
    fn test_rgb_display() {
        let rgb = RGB { r: 255, g: 85, b: 0 };
        assert_eq!(format!("{}", rgb), "#ff5500");

        let rgb = RGB { r: 0, g: 17, b: 255 };
        assert_eq!(format!("{}", rgb), "#0011ff");
    }

    #[test]
    fn test_rgb_format_as_hex() {
        let rgb = RGB { r: 255, g: 85, b: 0 };
        assert_eq!(rgb.format_as(crate::user_config::ColorFormat::Hex), "#ff5500");
    }

    #[test]
    fn test_rgb_format_as_rgb() {
        let rgb = RGB { r: 255, g: 85, b: 0 };
        assert_eq!(rgb.format_as(crate::user_config::ColorFormat::Rgb), "rgb(255, 85, 0)");
    }

    #[test]
    fn test_rgb_format_as_hsl() {
        let rgb = RGB { r: 255, g: 85, b: 0 };
        let result = rgb.format_as(crate::user_config::ColorFormat::Hsl);
        // HSL for #ff5500 is approximately hsl(20, 100%, 50%)
        assert!(result.contains("hsl("));
        assert!(result.contains("20"));
        assert!(result.contains("100%"));
        assert!(result.contains("50%"));
    }

    #[test]
    fn test_config_source_termtint() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join(".termtint");
        File::create(&config_path).unwrap();

        let user_config = UserConfig::default();
        let result = find_config_source(temp.path(), &user_config);

        assert_eq!(result, Some(ConfigSource::Termtint(config_path)));
    }

    #[test]
    fn test_config_source_trigger_file() {
        let temp = TempDir::new().unwrap();
        let trigger_path = temp.path().join("Cargo.toml");
        File::create(&trigger_path).unwrap();

        let mut user_config = UserConfig::default();
        user_config.trigger_files = vec!["Cargo.toml".to_string()];

        let result = find_config_source(temp.path(), &user_config);

        assert_eq!(
            result,
            Some(ConfigSource::TriggerFile(temp.path().to_string_lossy().to_string()))
        );
    }

    #[test]
    fn test_config_source_termtint_priority() {
        // .termtint should take priority over trigger files
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join(".termtint");
        File::create(&config_path).unwrap();
        let trigger_path = temp.path().join("package.json");
        File::create(&trigger_path).unwrap();

        let mut user_config = UserConfig::default();
        user_config.trigger_files = vec!["package.json".to_string()];

        let result = find_config_source(temp.path(), &user_config);

        assert_eq!(result, Some(ConfigSource::Termtint(config_path)));
    }

    #[test]
    fn test_config_source_multiple_trigger_files() {
        let temp = TempDir::new().unwrap();
        let trigger1 = temp.path().join("Cargo.toml");
        File::create(&trigger1).unwrap();
        let trigger2 = temp.path().join("package.json");
        File::create(&trigger2).unwrap();

        let mut user_config = UserConfig::default();
        user_config.trigger_files = vec!["pyproject.toml".to_string(), "Cargo.toml".to_string(), "package.json".to_string()];

        let result = find_config_source(temp.path(), &user_config);

        // Should match first trigger file in the list that exists
        assert_eq!(
            result,
            Some(ConfigSource::TriggerFile(temp.path().to_string_lossy().to_string()))
        );
    }

    #[test]
    fn test_config_source_parent_dir() {
        let temp = TempDir::new().unwrap();
        let trigger_path = temp.path().join("Cargo.toml");
        File::create(&trigger_path).unwrap();

        let child_dir = temp.path().join("child");
        fs::create_dir(&child_dir).unwrap();

        let mut user_config = UserConfig::default();
        user_config.trigger_files = vec!["Cargo.toml".to_string()];

        let result = find_config_source(&child_dir, &user_config);

        assert_eq!(
            result,
            Some(ConfigSource::TriggerFile(temp.path().to_string_lossy().to_string()))
        );
    }

    #[test]
    fn test_config_source_none() {
        let temp = TempDir::new().unwrap();
        let child_dir = temp.path().join("child");
        fs::create_dir(&child_dir).unwrap();

        let mut user_config = UserConfig::default();
        user_config.trigger_files = vec!["Cargo.toml".to_string()];

        let result = find_config_source(&child_dir, &user_config);

        assert_eq!(result, None);
    }

    #[test]
    fn test_config_source_empty_trigger_list() {
        let temp = TempDir::new().unwrap();
        let trigger_path = temp.path().join("Cargo.toml");
        File::create(&trigger_path).unwrap();

        let user_config = UserConfig::default(); // empty trigger_files

        let result = find_config_source(temp.path(), &user_config);

        // Should find nothing since trigger_files is empty
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_config_source_termtint() {
        let user_config = UserConfig::default();
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join(".termtint");
        fs::write(&config_path, "#ff5500").unwrap();

        let source = ConfigSource::Termtint(config_path);
        let config = parse_config_source(&source, &user_config).unwrap();

        assert_eq!(config.tab, RGB { r: 255, g: 85, b: 0 });
        // Background uses fixed lightness (0.10 by default)
        assert_eq!(config.background, RGB { r: 48, g: 0, b: 0 });
    }

    #[test]
    fn test_parse_config_source_trigger_file() {
        let user_config = UserConfig::default();
        let temp = TempDir::new().unwrap();

        let source = ConfigSource::TriggerFile(temp.path().to_string_lossy().to_string());
        let config = parse_config_source(&source, &user_config).unwrap();

        // Should generate auto color based on directory path
        // Verify it's a valid color with at least one bright channel
        let max_channel = config.tab.r.max(config.tab.g).max(config.tab.b);
        assert!(max_channel >= 128, "Generated color should be vibrant");
    }

    #[test]
    fn test_parse_config_source_trigger_file_deterministic() {
        let user_config = UserConfig::default();
        let temp = TempDir::new().unwrap();

        let source = ConfigSource::TriggerFile(temp.path().to_string_lossy().to_string());
        let config1 = parse_config_source(&source, &user_config).unwrap();
        let config2 = parse_config_source(&source, &user_config).unwrap();

        // Should generate the same color for the same directory
        assert_eq!(config1.tab, config2.tab);
        assert_eq!(config1.background, config2.background);
    }

    #[test]
    fn test_parse_config_source_trigger_file_different_dirs() {
        let user_config = UserConfig::default();
        let temp1 = TempDir::new().unwrap();
        let temp2 = TempDir::new().unwrap();

        let source1 = ConfigSource::TriggerFile(temp1.path().to_string_lossy().to_string());
        let source2 = ConfigSource::TriggerFile(temp2.path().to_string_lossy().to_string());

        let config1 = parse_config_source(&source1, &user_config).unwrap();
        let config2 = parse_config_source(&source2, &user_config).unwrap();

        // Different directories should generate different colors
        assert_ne!(config1.tab, config2.tab);
    }

    #[test]
    fn test_parse_config_source_uses_user_config() {
        let mut user_config = UserConfig::default();
        user_config.background_lightness = 0.20;

        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join(".termtint");
        fs::write(&config_path, "#ff5500").unwrap();

        let source = ConfigSource::Termtint(config_path);
        let config = parse_config_source(&source, &user_config).unwrap();

        // Should use custom background lightness value
        assert_eq!(config.tab, RGB { r: 255, g: 85, b: 0 });
        // Background should use fixed lightness of 0.20
        assert_eq!(config.background, RGB { r: 84, g: 0, b: 0 });
    }

    #[test]
    fn test_generate_random_color() {
        let user_config = UserConfig::default();
        let color = generate_random_color(&user_config);

        // Convert back to HSL to verify constraints
        let color_str = format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b);
        let parsed_color = csscolorparser::parse(&color_str).unwrap();
        let [_hue, saturation, lightness, _alpha] = parsed_color.to_hsla();

        // Verify saturation is within configured range (with tolerance for conversion)
        assert!(
            saturation >= user_config.saturation_min - 0.05,
            "Saturation {} should be >= {}",
            saturation,
            user_config.saturation_min
        );
        assert!(
            saturation <= user_config.saturation_max + 0.05,
            "Saturation {} should be <= {}",
            saturation,
            user_config.saturation_max
        );

        // Verify lightness matches configured value (with tolerance)
        assert!(
            (lightness - user_config.lightness).abs() < 0.02,
            "Lightness {} should be approximately {}",
            lightness,
            user_config.lightness
        );
    }

    #[test]
    fn test_generate_random_color_produces_different_colors() {
        let user_config = UserConfig::default();

        // Generate multiple random colors
        let colors: Vec<RGB> = (0..10).map(|_| generate_random_color(&user_config)).collect();

        // At least some should be different (highly unlikely all 10 are the same)
        let first_color = colors[0];
        let has_different_color = colors.iter().any(|c| *c != first_color);
        assert!(
            has_different_color,
            "Should generate different random colors, but all were {:?}",
            first_color
        );
    }

    #[test]
    fn test_generate_random_color_respects_custom_ranges() {
        let mut user_config = UserConfig::default();
        user_config.hue_min = 120.0; // Green range
        user_config.hue_max = 180.0; // Cyan range
        user_config.saturation_min = 0.8;
        user_config.saturation_max = 0.9;
        user_config.lightness = 0.6;

        let color = generate_random_color(&user_config);

        // Convert back to HSL to verify
        let color_str = format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b);
        let parsed_color = csscolorparser::parse(&color_str).unwrap();
        let [hue, saturation, lightness, _alpha] = parsed_color.to_hsla();

        // Verify hue is within configured range (with some tolerance for conversion)
        assert!(
            hue >= user_config.hue_min - 5.0 && hue <= user_config.hue_max + 5.0,
            "Hue {} should be within {} to {}",
            hue,
            user_config.hue_min,
            user_config.hue_max
        );

        // Verify saturation is within configured range
        assert!(
            saturation >= user_config.saturation_min - 0.05,
            "Saturation {} should be >= {}",
            saturation,
            user_config.saturation_min
        );

        // Verify lightness matches configured value
        assert!(
            (lightness - user_config.lightness).abs() < 0.02,
            "Lightness {} should be approximately {}",
            lightness,
            user_config.lightness
        );
    }
}
