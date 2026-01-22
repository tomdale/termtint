use crate::config::RGB;
use crate::user_config::UserConfig;
use csscolorparser;

/// Display a visual color palette showing available auto-generated colors.
///
/// # Arguments
/// * `user_config` - User configuration containing color generation parameters
pub fn cmd_colors(user_config: &UserConfig) {
    // Print header
    println!("termtint color palette\n");

    // Print current configuration values
    println!("Configuration:");
    println!("  Background lightness: {:.0}%", user_config.background_lightness * 100.0);
    println!("\nAuto color generation:");
    println!("  Hue range:        {:.0}° - {:.0}°", user_config.hue_min, user_config.hue_max);
    println!("  Saturation range: {:.0}% - {:.0}%", user_config.saturation_min * 100.0, user_config.saturation_max * 100.0);
    println!("  Lightness:        {:.0}%", user_config.lightness * 100.0);

    // Print hue spectrum
    println!("\nHue spectrum:");
    print_hue_spectrum(user_config);

    // Print sample tab/background pairs
    println!("\nSample tab/background pairs:");
    print_sample_pairs(user_config);
}

/// Print a visual hue spectrum using ANSI true color and Unicode blocks.
fn print_hue_spectrum(user_config: &UserConfig) {
    let steps = 36;
    let hue_range = user_config.hue_max - user_config.hue_min;

    // Use midpoint value for saturation and configured lightness
    let saturation = (user_config.saturation_min + user_config.saturation_max) / 2.0;
    let lightness = user_config.lightness;

    print!("  ");
    for i in 0..steps {
        let hue = user_config.hue_min + (i as f32 / steps as f32) * hue_range;
        let color = csscolorparser::Color::from_hsla(hue, saturation, lightness, 1.0);
        let [r, g, b, _a] = color.to_rgba8();

        // Print colored block using ANSI true color
        print!("\x1b[48;2;{};{};{}m \x1b[0m", r, g, b);
    }
    println!();
}

/// Print sample tab/background color pairs.
fn print_sample_pairs(user_config: &UserConfig) {
    let samples = 12;
    let hue_range = user_config.hue_max - user_config.hue_min;

    // Use midpoint value for saturation and configured lightness
    let saturation = (user_config.saturation_min + user_config.saturation_max) / 2.0;
    let lightness = user_config.lightness;

    for i in 0..samples {
        let hue = user_config.hue_min + (i as f32 / samples as f32) * hue_range;
        let color = csscolorparser::Color::from_hsla(hue, saturation, lightness, 1.0);
        let [r, g, b, _a] = color.to_rgba8();

        let tab = RGB { r, g, b };
        let background = tab.with_lightness(user_config.background_lightness);

        // Print colored blocks with hex values
        print!("  Tab: ");
        print!("\x1b[48;2;{};{};{}m   \x1b[0m", tab.r, tab.g, tab.b);
        print!(" {} ", tab);

        print!(" Bg: ");
        print!("\x1b[48;2;{};{};{}m   \x1b[0m", background.r, background.g, background.b);
        print!(" {}", background);

        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cmd_colors_runs_without_panic() {
        let user_config = UserConfig::default();
        // Just verify it doesn't panic
        cmd_colors(&user_config);
    }

    #[test]
    fn test_cmd_colors_with_custom_config() {
        let user_config = UserConfig {
            hue_min: 0.0,
            hue_max: 180.0,
            saturation_min: 0.5,
            saturation_max: 0.8,
            lightness: 0.45,
            background_lightness: 0.08,
            trigger_files: Vec::new(),
        };
        // Just verify it doesn't panic with custom config
        cmd_colors(&user_config);
    }
}
