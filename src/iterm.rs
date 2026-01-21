use crate::config::{ColorConfig, RGB};

/// Set the iTerm2 tab color using OSC 6 escape sequences.
pub fn set_tab_color(rgb: RGB) {
    // iTerm2 proprietary escape sequence for tab color
    print!("\x1b]6;1;bg;red;brightness;{}\x07", rgb.r);
    print!("\x1b]6;1;bg;green;brightness;{}\x07", rgb.g);
    print!("\x1b]6;1;bg;blue;brightness;{}\x07", rgb.b);
}

/// Set the terminal background color using OSC 11.
pub fn set_background_color(rgb: RGB) {
    // Standard OSC 11 for background color (hex format)
    print!(
        "\x1b]11;rgb:{:02x}/{:02x}/{:02x}\x07",
        rgb.r, rgb.g, rgb.b
    );
}

/// Reset the iTerm2 tab color to default.
pub fn reset_tab_color() {
    print!("\x1b]6;1;bg;*;default\x07");
}

/// Reset the terminal background color to default.
pub fn reset_background_color() {
    print!("\x1b]111\x07");
}

/// Apply both tab and background colors from a ColorConfig.
pub fn apply_colors(config: &ColorConfig) {
    set_tab_color(config.tab);
    set_background_color(config.background);
}

/// Reset both tab and background colors to defaults.
pub fn reset_colors() {
    reset_tab_color();
    reset_background_color();
}
