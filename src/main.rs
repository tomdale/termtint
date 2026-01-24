use clap::{Parser, Subcommand};

mod colors;
mod config;
mod init;
mod iterm;
mod state;
mod user_config;

#[derive(Parser)]
#[command(name = "termtint")]
#[command(about = "Terminal color theming based on directory")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Print shell hook code
    Hook {
        /// Shell type (zsh, bash, or fish)
        shell: String,
    },
    /// Apply colors from config in current directory
    Apply {
        /// Show detailed config info, color swatches, and status messages
        #[arg(short, long)]
        verbose: bool,
        /// Force apply even if config is unchanged
        #[arg(short, long)]
        force: bool,
    },
    /// Reset terminal colors to default
    Reset {
        /// Show escape sequences, state file info, and previous configuration
        #[arg(short, long)]
        verbose: bool,
    },
    /// Initialize a .termtint file in the current directory
    Init {
        /// Hex color for the tab (e.g., #ff5500)
        color: Option<String>,
        /// Custom background color (hex)
        #[arg(long)]
        background: Option<String>,
        /// Overwrite existing .termtint file
        #[arg(short, long)]
        force: bool,
    },
    /// Re-roll to a new random color, updating .termtint in current directory
    Reroll {
        /// Show directory path
        #[arg(short, long)]
        verbose: bool,
    },
    /// Display visual color palette and configuration
    Colors,
    /// Show current configuration and config file path
    Config {
        /// Open config file in editor
        #[arg(short, long)]
        edit: bool,
        /// Print config file path only
        #[arg(short, long)]
        path: bool,
    },
    /// Show color configuration details for current directory
    Inspect,
    /// Manage triggers for auto-generated colors
    Trigger {
        #[command(subcommand)]
        action: TriggerAction,
    },
}

#[derive(Subcommand)]
enum TriggerAction {
    /// Add a trigger (file name or path glob)
    Add {
        /// Pattern to add - file name (e.g., Cargo.toml) or path glob (e.g., ~/Code/*)
        pattern: String,
    },
    /// Remove a trigger (file name or path glob)
    Remove {
        /// Pattern to remove
        pattern: String,
    },
    /// List all triggers
    List,
}

/// Print both color swatches in a unified bordered box to stdout.
/// Used by cmd_inspect() to display colors with proper margins.
fn print_color_swatches_stdout(
    tab: &config::RGB,
    background: &config::RGB,
    user_config: &user_config::UserConfig,
) {
    let swatch_width = 16;
    let swatch_height = 6;
    let left_margin = 3;
    let between_swatches = 8;
    let right_margin = 3;

    // Total inner width for the box
    let inner_width = left_margin + swatch_width + between_swatches + swatch_width + right_margin;

    // Unicode double-line box drawing characters
    let top_left = '╔';
    let top_right = '╗';
    let bottom_left = '╚';
    let bottom_right = '╝';
    let horizontal = '═';
    let vertical = '║';

    // Format color strings for display
    let tab_str = tab.format_as(user_config.color_format);
    let bg_str = background.format_as(user_config.color_format);

    // Top border
    print!("{}", top_left);
    for _ in 0..inner_width {
        print!("{}", horizontal);
    }
    println!("{}", top_right);

    // Black background for box interior
    let black_bg = "\x1b[48;2;0;0;0m";

    // Empty margin row
    print!("{}{}", vertical, black_bg);
    for _ in 0..inner_width {
        print!(" ");
    }
    println!("\x1b[0m{}", vertical);

    // Label row
    print!("{}{}", vertical, black_bg);
    print!("{:width$}", "", width = left_margin);
    print!("{:<width$}", "Tab:", width = swatch_width);
    print!("{:width$}", "", width = between_swatches);
    print!("{:<width$}", "Background:", width = swatch_width);
    print!("{:width$}", "", width = right_margin);
    println!("\x1b[0m{}", vertical);

    // Empty margin row
    print!("{}{}", vertical, black_bg);
    for _ in 0..inner_width {
        print!(" ");
    }
    println!("\x1b[0m{}", vertical);

    // Swatch rows
    for _ in 0..swatch_height {
        print!("{}{}", vertical, black_bg);

        // Left margin
        for _ in 0..left_margin {
            print!(" ");
        }

        // Tab color swatch
        print!("\x1b[48;2;{};{};{}m", tab.r, tab.g, tab.b);
        for _ in 0..swatch_width {
            print!(" ");
        }
        print!("{}", black_bg);

        // Between swatches
        for _ in 0..between_swatches {
            print!(" ");
        }

        // Background color swatch
        print!(
            "\x1b[48;2;{};{};{}m",
            background.r, background.g, background.b
        );
        for _ in 0..swatch_width {
            print!(" ");
        }
        print!("{}", black_bg);

        // Right margin
        for _ in 0..right_margin {
            print!(" ");
        }

        println!("\x1b[0m{}", vertical);
    }

    // Empty margin row
    print!("{}{}", vertical, black_bg);
    for _ in 0..inner_width {
        print!(" ");
    }
    println!("\x1b[0m{}", vertical);

    // Color value row
    print!("{}{}", vertical, black_bg);
    print!("{:width$}", "", width = left_margin);
    print!("{:<width$}", tab_str, width = swatch_width);
    print!("{:width$}", "", width = between_swatches);
    print!("{:<width$}", bg_str, width = swatch_width);
    print!("{:width$}", "", width = right_margin);
    println!("\x1b[0m{}", vertical);

    // Empty margin row
    print!("{}{}", vertical, black_bg);
    for _ in 0..inner_width {
        print!(" ");
    }
    println!("\x1b[0m{}", vertical);

    // Bottom border
    print!("{}", bottom_left);
    for _ in 0..inner_width {
        print!("{}", horizontal);
    }
    println!("{}", bottom_right);
}

fn print_color_swatches(
    tab: &config::RGB,
    background: &config::RGB,
    user_config: &user_config::UserConfig,
) {
    let swatch_width = 16;
    let swatch_height = 6;
    let left_margin = 3;
    let between_swatches = 8;
    let right_margin = 3;

    // Total inner width for the box
    let inner_width = left_margin + swatch_width + between_swatches + swatch_width + right_margin;

    // Unicode double-line box drawing characters
    let top_left = '╔';
    let top_right = '╗';
    let bottom_left = '╚';
    let bottom_right = '╝';
    let horizontal = '═';
    let vertical = '║';

    // Format color strings for display
    let tab_str = tab.format_as(user_config.color_format);
    let bg_str = background.format_as(user_config.color_format);

    eprintln!();

    // Top border
    eprint!("{}", top_left);
    for _ in 0..inner_width {
        eprint!("{}", horizontal);
    }
    eprintln!("{}", top_right);

    // Black background for box interior
    let black_bg = "\x1b[48;2;0;0;0m";

    // Empty margin row
    eprint!("{}{}", vertical, black_bg);
    for _ in 0..inner_width {
        eprint!(" ");
    }
    eprintln!("\x1b[0m{}", vertical);

    // Label row
    eprint!("{}{}", vertical, black_bg);
    eprint!("{:width$}", "", width = left_margin);
    eprint!("{:<width$}", "Tab:", width = swatch_width);
    eprint!("{:width$}", "", width = between_swatches);
    eprint!("{:<width$}", "Background:", width = swatch_width);
    eprint!("{:width$}", "", width = right_margin);
    eprintln!("\x1b[0m{}", vertical);

    // Empty margin row
    eprint!("{}{}", vertical, black_bg);
    for _ in 0..inner_width {
        eprint!(" ");
    }
    eprintln!("\x1b[0m{}", vertical);

    // Swatch rows
    for _ in 0..swatch_height {
        eprint!("{}{}", vertical, black_bg);

        // Left margin
        for _ in 0..left_margin {
            eprint!(" ");
        }

        // Tab color swatch
        eprint!("\x1b[48;2;{};{};{}m", tab.r, tab.g, tab.b);
        for _ in 0..swatch_width {
            eprint!(" ");
        }
        eprint!("{}", black_bg);

        // Between swatches
        for _ in 0..between_swatches {
            eprint!(" ");
        }

        // Background color swatch
        eprint!(
            "\x1b[48;2;{};{};{}m",
            background.r, background.g, background.b
        );
        for _ in 0..swatch_width {
            eprint!(" ");
        }
        eprint!("{}", black_bg);

        // Right margin
        for _ in 0..right_margin {
            eprint!(" ");
        }

        eprintln!("\x1b[0m{}", vertical);
    }

    // Empty margin row
    eprint!("{}{}", vertical, black_bg);
    for _ in 0..inner_width {
        eprint!(" ");
    }
    eprintln!("\x1b[0m{}", vertical);

    // Color value row
    eprint!("{}{}", vertical, black_bg);
    eprint!("{:width$}", "", width = left_margin);
    eprint!("{:<width$}", tab_str, width = swatch_width);
    eprint!("{:width$}", "", width = between_swatches);
    eprint!("{:<width$}", bg_str, width = swatch_width);
    eprint!("{:width$}", "", width = right_margin);
    eprintln!("\x1b[0m{}", vertical);

    // Empty margin row
    eprint!("{}{}", vertical, black_bg);
    for _ in 0..inner_width {
        eprint!(" ");
    }
    eprintln!("\x1b[0m{}", vertical);

    // Bottom border
    eprint!("{}", bottom_left);
    for _ in 0..inner_width {
        eprint!("{}", horizontal);
    }
    eprintln!("{}", bottom_right);
}

fn print_config_info(
    source: &config::ConfigSource,
    color_config: &config::ColorConfig,
    user_config: &user_config::UserConfig,
) {
    eprintln!("Config Information:");
    eprintln!();

    // 1. Config source type and path
    match source {
        config::ConfigSource::Termtint(path) => {
            eprintln!("Source type:     Explicit .termtint file");
            eprintln!("Source path:     {}", path.display());

            // Read the file content to detect format and raw values
            if let Ok(content) = std::fs::read_to_string(path) {
                let format = config::detect_format(&content);
                let format_str = match format {
                    config::ConfigFormat::Auto => "auto",
                    config::ConfigFormat::SimpleColor => "simple (hex color)",
                    config::ConfigFormat::Toml => "toml",
                };
                eprintln!("Config format:   {}", format_str);
                eprintln!();

                eprintln!("Raw config:");
                for line in content.lines() {
                    eprintln!("  {}", line);
                }
                eprintln!();

                // Determine if background is auto-generated or explicit
                let background_explicit = match format {
                    config::ConfigFormat::Toml => {
                        // Check if TOML contains explicit background key
                        content.contains("background")
                    }
                    _ => false,
                };

                if background_explicit {
                    eprintln!("Background:      Explicit (defined in config)");
                } else {
                    eprintln!(
                        "Background:      Auto-generated ({}% lightness)",
                        (user_config.background_lightness * 100.0) as u8
                    );
                }
            }
        }
        config::ConfigSource::TriggerFile(dir_path) => {
            eprintln!("Source type:     Trigger file (auto-generated color)");
            eprintln!("Source path:     {}", dir_path);
            eprintln!("Config format:   auto (hash-based)");
            eprintln!();
            eprintln!("Raw config:      <auto-generated from directory path>");
            eprintln!();
            eprintln!(
                "Background:      Auto-generated ({}% lightness)",
                (user_config.background_lightness * 100.0) as u8
            );
        }
        config::ConfigSource::TriggerPath(dir_path) => {
            eprintln!("Source type:     Trigger path (auto-generated color)");
            eprintln!("Source path:     {}", dir_path);
            eprintln!("Config format:   auto (hash-based)");
            eprintln!();
            eprintln!("Raw config:      <auto-generated from directory path>");
            eprintln!();
            eprintln!(
                "Background:      Auto-generated ({}% lightness)",
                (user_config.background_lightness * 100.0) as u8
            );
        }
    }

    eprintln!();
    eprintln!("Resolved colors:");
    eprintln!(
        "  Tab:           {}",
        color_config.tab.format_as(user_config.color_format)
    );
    eprintln!(
        "  Background:    {}",
        color_config.background.format_as(user_config.color_format)
    );
    eprintln!();
}

fn cmd_apply(verbose: bool, force: bool) {
    state::cleanup_stale_sessions();

    let user_config = user_config::load_user_config();

    let current_dir = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("Error getting current directory: {}", e);
            return;
        }
    };

    let config_source = config::find_config_source(&current_dir, &user_config);
    let last_state = state::read_last_config_state();

    // Build current state if we have a config source
    let current_state = config_source.as_ref().and_then(|source| match source {
        config::ConfigSource::Termtint(path) => {
            // For explicit .termtint files, track the file's mtime
            state::get_file_mtime(path).map(|mtime| state::ConfigState {
                path: path.clone(),
                mtime,
                source_type: state::ConfigSourceType::Explicit,
            })
        }
        config::ConfigSource::TriggerPath(dir_path) => {
            // For path globs, use directory path and always mtime 0 (always apply)
            Some(state::ConfigState {
                path: std::path::PathBuf::from(dir_path),
                mtime: 0,
                source_type: state::ConfigSourceType::TriggerPath,
            })
        }
        config::ConfigSource::TriggerFile(dir_path) => {
            // For trigger files, use directory path and always mtime 0 (always apply)
            Some(state::ConfigState {
                path: std::path::PathBuf::from(dir_path),
                mtime: 0,
                source_type: state::ConfigSourceType::TriggerFile,
            })
        }
    });

    match (&current_state, &last_state) {
        // Same config source and unchanged, no change needed (skip if force is set)
        (Some(current), Some(last)) if current == last && !force => {
            if verbose {
                if let Some(source) = &config_source {
                    if let Ok(color_config) = config::parse_config_source(source, &user_config) {
                        print_config_info(source, &color_config, &user_config);
                        eprintln!("termtint: (unchanged)");
                        print_color_swatches(
                            &color_config.tab,
                            &color_config.background,
                            &user_config,
                        );
                    }
                }
            }
        }

        // Found a config source (new or changed)
        (Some(current), _) => {
            if let Some(source) = &config_source {
                match config::parse_config_source(source, &user_config) {
                    Ok(color_config) => {
                        if verbose {
                            print_config_info(source, &color_config, &user_config);
                            eprintln!("termtint: applying colors");
                            print_color_swatches(
                                &color_config.tab,
                                &color_config.background,
                                &user_config,
                            );
                        }
                        iterm::apply_colors(&color_config);
                        state::write_last_config_state(Some(current));
                    }
                    Err(e) => {
                        eprintln!("Error parsing config: {}", e);
                    }
                }
            }
        }

        // No config found, but had one before - reset colors
        (None, Some(_)) => {
            if verbose {
                eprintln!("termtint: reset (no config)");
            }
            iterm::reset_colors();
            state::write_last_config_state(None);
        }

        // No config found and none before - reset to ensure clean state
        (None, None) => {
            if verbose {
                eprintln!("termtint: reset (no config found)");
            }
            iterm::reset_colors();
        }
    }
}

fn cmd_reset(verbose: bool) {
    if verbose {
        eprintln!("termtint: resetting colors to default");
        eprintln!();

        // Show the escape sequences being emitted
        eprintln!("Escape sequences:");
        let (tab_seq, bg_seq) = iterm::get_reset_sequences();
        eprintln!("  Tab color reset:        {}", escape_for_display(&tab_seq));
        eprintln!("  Background color reset: {}", escape_for_display(&bg_seq));
        eprintln!();

        // Show state file information
        let state_path = state::state_file_path();
        eprintln!("State file: {}", state_path.display());

        let last_state = state::read_last_config_state();
        match last_state {
            Some(state) => {
                eprintln!("Previous state:");
                eprintln!("  Path: {}", state.path.display());
                eprintln!("  Modified time: {}", state.mtime);
                eprintln!("  Source type: {:?}", state.source_type);
                eprintln!();
                eprintln!("Clearing state file...");
            }
            None => {
                eprintln!("Previous state: none");
                eprintln!();
            }
        }
    }

    iterm::reset_colors();
    state::write_last_config_state(None);

    if verbose {
        eprintln!("Done.");
    }
}

/// Convert escape sequences to a readable format for display
fn escape_for_display(seq: &str) -> String {
    seq.replace('\x1b', "\\x1b").replace('\x07', "\\x07")
}

fn cmd_hook(shell: &str) {
    match shell {
        "zsh" => {
            println!(
                r#"_termtint_hook() {{
  termtint apply
}}
autoload -Uz add-zsh-hook
add-zsh-hook chpwd _termtint_hook
_termtint_hook"#
            );
        }
        "bash" => {
            println!(
                r#"_termtint_hook() {{
  termtint apply
}}
_termtint_prompt_command() {{
  local _termtint_new_pwd="$PWD"
  if [[ "$_termtint_new_pwd" != "$_TERMTINT_LAST_PWD" ]]; then
    _TERMTINT_LAST_PWD="$_termtint_new_pwd"
    _termtint_hook
  fi
}}
_TERMTINT_LAST_PWD="$PWD"
if [[ -z "${{PROMPT_COMMAND}}" ]]; then
  PROMPT_COMMAND="_termtint_prompt_command"
elif [[ "${{PROMPT_COMMAND}}" != *"_termtint_prompt_command"* ]]; then
  PROMPT_COMMAND="_termtint_prompt_command;${{PROMPT_COMMAND}}"
fi
_termtint_hook"#
            );
        }
        "fish" => {
            println!(
                r#"function _termtint_hook --on-variable PWD
  termtint apply
end
_termtint_hook"#
            );
        }
        _ => {
            eprintln!("Error: unsupported shell '{}'. Supported shells: zsh, bash, fish", shell);
            std::process::exit(1);
        }
    }
}

fn cmd_config(user_config: &user_config::UserConfig) {
    let config_path = user_config::config_file_path();
    let exists = config_path.exists();

    println!("Config file: {}", config_path.display());
    if exists {
        println!("Status: exists");
    } else {
        println!("Status: not found (using defaults)");
    }

    println!("\n{}", "=".repeat(60));
    println!("AVAILABLE SETTINGS");
    println!("{}", "=".repeat(60));

    // background_lightness
    println!(
        "\nbackground_lightness = {:.2}",
        user_config.background_lightness
    );
    println!("  Lightness for auto-darkened backgrounds.");
    println!("  Range: 0.0 (black) to 1.0 (full brightness)");
    println!("  Default: 0.18");

    // background_saturation
    println!(
        "\nbackground_saturation = {:.2}",
        user_config.background_saturation
    );
    println!("  Saturation multiplier for auto-darkened backgrounds.");
    println!("  Range: 0.0 (grayscale) to 1.0 (preserve original)");
    println!("  Default: 1.00");

    // trigger_files
    if user_config.trigger_files.is_empty() {
        println!("\ntrigger_files = []");
    } else {
        println!("\ntrigger_files = {:?}", user_config.trigger_files);
    }
    println!("  Files that trigger automatic color generation when found.");
    println!("  When present in a directory, termtint generates a hash-based color.");
    println!("  Example: [\"Cargo.toml\", \"package.json\", \"go.mod\"]");
    println!("  Default: [] (disabled)");

    // trigger_paths
    if user_config.trigger_paths.is_empty() {
        println!("\ntrigger_paths = []");
    } else {
        println!("\ntrigger_paths = {:?}", user_config.trigger_paths);
    }
    println!("  Path globs that trigger automatic color generation.");
    println!("  Directories matching these patterns are treated as having 'auto' .termtint.");
    println!("  Supports ~ for home directory. Example: [\"~/Code/*\", \"~/Projects/*\"]");
    println!("  Default: [] (disabled)");

    // color_format
    let format_str = match user_config.color_format {
        user_config::ColorFormat::Hex => "hex",
        user_config::ColorFormat::Hsl => "hsl",
        user_config::ColorFormat::Rgb => "rgb",
    };
    println!("\ncolor_format = \"{}\"", format_str);
    println!("  Format for displaying colors in output.");
    println!(
        "  Options: \"hex\" (#ff5500), \"hsl\" (hsl(20, 100%, 50%)), \"rgb\" (rgb(255, 85, 0))"
    );
    println!("  Default: \"hex\"");

    println!("\n{}", "-".repeat(60));
    println!("[auto] - Auto Color Generation Parameters");
    println!("{}", "-".repeat(60));

    // hue_min / hue_max
    println!("\nhue_min = {:.1}", user_config.hue_min);
    println!("hue_max = {:.1}", user_config.hue_max);
    println!("  Hue range for auto-generated colors (color wheel position).");
    println!("  Range: 0.0 to 360.0 (degrees)");
    println!("  0=red, 60=yellow, 120=green, 180=cyan, 240=blue, 300=magenta");
    println!("  Default: 0.0 - 360.0 (full spectrum)");

    // saturation_min / saturation_max
    println!("\nsaturation_min = {:.2}", user_config.saturation_min);
    println!("saturation_max = {:.2}", user_config.saturation_max);
    println!("  Saturation range for auto-generated colors (color intensity).");
    println!("  Range: 0.0 (gray) to 1.0 (vivid)");
    println!("  Default: 0.7 - 0.9");

    // lightness
    println!("\nlightness = {:.2}", user_config.lightness);
    println!("  Lightness for auto-generated tab colors.");
    println!("  Range: 0.0 (dark) to 1.0 (bright)");
    println!("  Default: 0.55");

    println!("\n{}", "=".repeat(60));
    println!("Run 'termtint config --edit' to edit your config file.");
}

fn cmd_inspect() {
    let user_config = user_config::load_user_config();

    let current_dir = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("Error getting current directory: {}", e);
            return;
        }
    };

    println!("Current directory: {}", current_dir.display());
    println!();

    // Find config source
    let config_source = config::find_config_source(&current_dir, &user_config);

    match &config_source {
        Some(config::ConfigSource::Termtint(path)) => {
            println!("Config source: .termtint file");
            println!("  Path: {}", path.display());
        }
        Some(config::ConfigSource::TriggerFile(dir_path)) => {
            println!("Config source: trigger file");
            println!("  Directory: {}", dir_path);

            // Determine which trigger file was matched
            let dir = std::path::PathBuf::from(dir_path);
            for trigger_file in &user_config.trigger_files {
                if dir.join(trigger_file).exists() {
                    println!("  Matched file: {}", trigger_file);
                    break;
                }
            }
        }
        Some(config::ConfigSource::TriggerPath(dir_path)) => {
            println!("Config source: trigger path");
            println!("  Directory: {}", dir_path);
        }
        None => {
            println!("Config source: none found");
        }
    }
    println!();

    // Parse and display colors if a config source was found
    if let Some(source) = &config_source {
        match config::parse_config_source(source, &user_config) {
            Ok(color_config) => {
                println!("Resolved colors:");
                print_color_swatches_stdout(
                    &color_config.tab,
                    &color_config.background,
                    &user_config,
                );
            }
            Err(e) => {
                println!("Error parsing config: {}", e);
            }
        }
        println!();
    }

    // Display cached state
    let last_state = state::read_last_config_state();
    match last_state {
        Some(state) => {
            println!("Cached state:");
            println!("  Path: {}", state.path.display());
            println!("  Modified time: {}", state.mtime);
            println!("  Source type: {:?}", state.source_type);
        }
        None => {
            println!("Cached state: none");
        }
    }
}

/// Returns true if the pattern looks like a path glob (contains /, *, ~, or ?)
fn is_path_pattern(pattern: &str) -> bool {
    pattern.contains('/') || pattern.contains('*') || pattern.contains('~') || pattern.contains('?')
}

fn cmd_trigger_add(pattern: &str) -> Result<(), String> {
    let mut user_config = user_config::load_user_config();

    if is_path_pattern(pattern) {
        // It's a path glob
        if user_config.trigger_paths.contains(&pattern.to_string()) {
            println!("'{}' is already in trigger paths.", pattern);
            return Ok(());
        }
        user_config.trigger_paths.push(pattern.to_string());
        user_config::save_trigger_paths(&user_config.trigger_paths)?;
        println!("Added '{}' to trigger paths.", pattern);
    } else {
        // It's a file name
        if user_config.trigger_files.contains(&pattern.to_string()) {
            println!("'{}' is already in trigger files.", pattern);
            return Ok(());
        }
        user_config.trigger_files.push(pattern.to_string());
        user_config::save_trigger_files(&user_config.trigger_files)?;
        println!("Added '{}' to trigger files.", pattern);
    }
    Ok(())
}

fn cmd_trigger_remove(pattern: &str) -> Result<(), String> {
    let mut user_config = user_config::load_user_config();

    // Check both lists and remove from whichever contains it
    let in_files = user_config.trigger_files.contains(&pattern.to_string());
    let in_paths = user_config.trigger_paths.contains(&pattern.to_string());

    if !in_files && !in_paths {
        println!("'{}' is not in triggers.", pattern);
        return Ok(());
    }

    if in_files {
        user_config.trigger_files.retain(|f| f != pattern);
        user_config::save_trigger_files(&user_config.trigger_files)?;
        println!("Removed '{}' from trigger files.", pattern);
    }

    if in_paths {
        user_config.trigger_paths.retain(|p| p != pattern);
        user_config::save_trigger_paths(&user_config.trigger_paths)?;
        println!("Removed '{}' from trigger paths.", pattern);
    }

    Ok(())
}

fn cmd_trigger_list(user_config: &user_config::UserConfig) {
    let has_files = !user_config.trigger_files.is_empty();
    let has_paths = !user_config.trigger_paths.is_empty();

    if !has_files && !has_paths {
        println!("No triggers configured.");
        return;
    }

    if has_files {
        println!("Trigger files:");
        for file in &user_config.trigger_files {
            println!("  {}", file);
        }
    }

    if has_files && has_paths {
        println!();
    }

    if has_paths {
        println!("Trigger paths:");
        for path in &user_config.trigger_paths {
            println!("  {}", path);
        }
    }
}

fn cmd_config_edit() -> Result<(), String> {
    // 1. Get config file path
    let config_path = user_config::config_file_path();

    // 2. Create parent directories if needed
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Error creating config directory: {}", e))?;
    }

    // 3. Create or upgrade config file
    if !config_path.exists() {
        // File doesn't exist - create with full defaults
        std::fs::write(&config_path, user_config::default_config_toml())
            .map_err(|e| format!("Error creating config file: {}", e))?;
    } else {
        // File exists - upgrade with any missing fields (added as comments)
        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| format!("Error reading config file: {}", e))?;
        let upgraded = user_config::upgrade_config(&content);
        if upgraded != content {
            std::fs::write(&config_path, &upgraded)
                .map_err(|e| format!("Error upgrading config file: {}", e))?;
        }
    }

    // 4. Read EDITOR environment variable and split into command + args
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    let mut parts = editor.split_whitespace();
    let cmd = parts.next().unwrap_or("vi");
    let args: Vec<&str> = parts.collect();

    // 5. Spawn editor process
    let status = std::process::Command::new(cmd)
        .args(&args)
        .arg(&config_path)
        .status()
        .map_err(|e| format!("Error launching editor '{}': {}", editor, e))?;

    // 6. Check if editor exited successfully
    if !status.success() {
        return Err(format!("Editor exited with status: {}", status));
    }

    Ok(())
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Hook { shell } => {
            cmd_hook(&shell);
        }
        Commands::Apply { verbose, force } => {
            cmd_apply(verbose, force);
        }
        Commands::Reset { verbose } => {
            cmd_reset(verbose);
        }
        Commands::Init {
            color,
            background,
            force,
        } => {
            let user_config = user_config::load_user_config();
            if let Err(e) = init::cmd_init(color, background, force, &user_config) {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
        Commands::Reroll { verbose } => {
            let user_config = user_config::load_user_config();
            if let Err(e) = init::cmd_reroll(verbose, &user_config) {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
        Commands::Colors => {
            let user_config = user_config::load_user_config();
            colors::cmd_colors(&user_config);
        }
        Commands::Config { edit, path } => {
            if path {
                println!("{}", user_config::config_file_path().display());
                return;
            }
            if edit {
                if let Err(e) = cmd_config_edit() {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            } else {
                let user_config = user_config::load_user_config();
                cmd_config(&user_config);
            }
        }
        Commands::Inspect => {
            cmd_inspect();
        }
        Commands::Trigger { action } => match action {
            TriggerAction::Add { pattern } => {
                if let Err(e) = cmd_trigger_add(&pattern) {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            }
            TriggerAction::Remove { pattern } => {
                if let Err(e) = cmd_trigger_remove(&pattern) {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            }
            TriggerAction::List => {
                let user_config = user_config::load_user_config();
                cmd_trigger_list(&user_config);
            }
        },
    }
}
