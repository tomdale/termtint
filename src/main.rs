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
        /// Shell type (zsh, bash, fish)
        shell: String,
    },
    /// Apply colors from config in current directory
    Apply {
        /// Show color swatches and status messages
        #[arg(short, long)]
        verbose: bool,
        /// Force apply even if config is unchanged
        #[arg(short, long)]
        force: bool,
        /// Show detailed config information including source path, format, and raw config
        #[arg(long)]
        info: bool,
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
        /// Create .termtint if it doesn't exist
        #[arg(short, long)]
        force: bool,
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
    /// Manage trigger files for auto-generated colors
    Trigger {
        #[command(subcommand)]
        action: TriggerAction,
    },
}

#[derive(Subcommand)]
enum TriggerAction {
    /// Add a file to trigger list
    Add {
        /// Filename to add (e.g., Cargo.toml)
        filename: String,
    },
    /// Remove a file from trigger list
    Remove {
        /// Filename to remove
        filename: String,
    },
    /// List all trigger files
    List,
}

fn print_color_swatches(tab: &config::RGB, background: &config::RGB, user_config: &user_config::UserConfig) {
    let swatch_height = 6;
    let swatch_width = 12;

    eprintln!("\nTab Color:              Background Color:");

    for _ in 0..swatch_height {
        // Print tab color swatch
        eprint!("\x1b[48;2;{};{};{}m", tab.r, tab.g, tab.b);
        for _ in 0..swatch_width {
            eprint!(" ");
        }
        eprint!("\x1b[0m");

        eprint!("      ");

        // Print background color swatch
        eprint!("\x1b[48;2;{};{};{}m", background.r, background.g, background.b);
        for _ in 0..swatch_width {
            eprint!(" ");
        }
        eprint!("\x1b[0m");

        eprintln!();
    }

    eprintln!("{}              {}",
        tab.format_as(user_config.color_format),
        background.format_as(user_config.color_format));
}

fn print_config_info(source: &config::ConfigSource, color_config: &config::ColorConfig, user_config: &user_config::UserConfig) {
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
                    eprintln!("Background:      Auto-generated ({}% lightness)", (user_config.background_lightness * 100.0) as u8);
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
            eprintln!("Background:      Auto-generated ({}% lightness)", (user_config.background_lightness * 100.0) as u8);
        }
    }

    eprintln!();
    eprintln!("Resolved colors:");
    eprintln!("  Tab:           {}", color_config.tab.format_as(user_config.color_format));
    eprintln!("  Background:    {}", color_config.background.format_as(user_config.color_format));
    eprintln!();
}

fn cmd_apply(verbose: bool, force: bool, info: bool) {
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
    let current_state = config_source.as_ref().map(|source| {
        match source {
            config::ConfigSource::Termtint(path) => {
                // For explicit .termtint files, track the file's mtime
                state::get_file_mtime(path).map(|mtime| state::ConfigState {
                    path: path.clone(),
                    mtime,
                    source_type: state::ConfigSourceType::Explicit,
                })
            }
            config::ConfigSource::TriggerFile(dir_path) => {
                // For trigger files, use directory path and always mtime 0 (always apply)
                Some(state::ConfigState {
                    path: std::path::PathBuf::from(dir_path),
                    mtime: 0,
                    source_type: state::ConfigSourceType::Triggered,
                })
            }
        }
    }).flatten();

    match (&current_state, &last_state) {
        // Same config source and unchanged, no change needed (skip if force is set)
        (Some(current), Some(last)) if current == last && !force => {
            if info || verbose {
                if let Some(source) = &config_source {
                    if let Ok(color_config) = config::parse_config_source(source, &user_config) {
                        if info {
                            print_config_info(source, &color_config, &user_config);
                        }
                        if verbose {
                            eprintln!("termtint: (unchanged)");
                            print_color_swatches(&color_config.tab, &color_config.background, &user_config);
                        }
                    }
                }
            }
        }

        // Found a config source (new or changed)
        (Some(current), _) => {
            if let Some(source) = &config_source {
                match config::parse_config_source(source, &user_config) {
                    Ok(color_config) => {
                        if info {
                            print_config_info(source, &color_config, &user_config);
                        }
                        if verbose {
                            eprintln!("termtint: applying colors");
                            print_color_swatches(&color_config.tab, &color_config.background, &user_config);
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
    seq.replace('\x1b', "\\x1b")
       .replace('\x07', "\\x07")
}

fn cmd_hook(shell: &str) {
    if shell != "zsh" {
        eprintln!("Error: only zsh is currently supported");
        std::process::exit(1);
    }

    // Output zsh hook script
    println!(
        r#"_termtint_hook() {{
  termtint apply
}}
autoload -Uz add-zsh-hook
add-zsh-hook chpwd _termtint_hook
_termtint_hook"#
    );
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
    println!("\nbackground_lightness = {:.2}", user_config.background_lightness);
    println!("  Lightness for auto-darkened backgrounds.");
    println!("  Range: 0.0 (black) to 1.0 (full brightness)");
    println!("  Default: 0.10");

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

    // color_format
    let format_str = match user_config.color_format {
        user_config::ColorFormat::Hex => "hex",
        user_config::ColorFormat::Hsl => "hsl",
        user_config::ColorFormat::Rgb => "rgb",
    };
    println!("\ncolor_format = \"{}\"", format_str);
    println!("  Format for displaying colors in output.");
    println!("  Options: \"hex\" (#ff5500), \"hsl\" (hsl(20, 100%, 50%)), \"rgb\" (rgb(255, 85, 0))");
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
                println!("  Tab color:        {} {}",
                    color_config.tab.format_as(user_config.color_format),
                    color_config.tab.as_color_block());
                println!("  Background color: {} {}",
                    color_config.background.format_as(user_config.color_format),
                    color_config.background.as_color_block());
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

fn cmd_trigger_add(filename: &str) -> Result<(), String> {
    let mut user_config = user_config::load_user_config();

    if user_config.trigger_files.contains(&filename.to_string()) {
        println!("'{}' is already in trigger files.", filename);
        return Ok(());
    }

    user_config.trigger_files.push(filename.to_string());
    user_config::save_trigger_files(&user_config.trigger_files)?;
    println!("Added '{}' to trigger files.", filename);
    Ok(())
}

fn cmd_trigger_remove(filename: &str) -> Result<(), String> {
    let mut user_config = user_config::load_user_config();

    if !user_config.trigger_files.contains(&filename.to_string()) {
        println!("'{}' is not in trigger files.", filename);
        return Ok(());
    }

    user_config.trigger_files.retain(|f| f != filename);
    user_config::save_trigger_files(&user_config.trigger_files)?;
    println!("Removed '{}' from trigger files.", filename);
    Ok(())
}

fn cmd_trigger_list(user_config: &user_config::UserConfig) {
    if user_config.trigger_files.is_empty() {
        println!("No trigger files configured.");
    } else {
        println!("Trigger files:");
        for file in &user_config.trigger_files {
            println!("  - {}", file);
        }
    }
}

fn cmd_config_edit() -> Result<(), String> {
    // 1. Get config file path
    let config_path = user_config::config_file_path();

    // 2. If file doesn't exist, create it with defaults
    if !config_path.exists() {
        // Create parent directories
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Error creating config directory: {}", e))?;
        }

        // Write default config
        std::fs::write(&config_path, user_config::default_config_toml())
            .map_err(|e| format!("Error creating config file: {}", e))?;
    }

    // 3. Read EDITOR environment variable and split into command + args
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    let mut parts = editor.split_whitespace();
    let cmd = parts.next().unwrap_or("vi");
    let args: Vec<&str> = parts.collect();

    // 4. Spawn editor process
    let status = std::process::Command::new(cmd)
        .args(&args)
        .arg(&config_path)
        .status()
        .map_err(|e| format!("Error launching editor '{}': {}", editor, e))?;

    // 5. Check if editor exited successfully
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
        Commands::Apply { verbose, force, info } => {
            cmd_apply(verbose, force, info);
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
        Commands::Reroll { force } => {
            let user_config = user_config::load_user_config();
            if let Err(e) = init::cmd_reroll(force, &user_config) {
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
            TriggerAction::Add { filename } => {
                if let Err(e) = cmd_trigger_add(&filename) {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            }
            TriggerAction::Remove { filename } => {
                if let Err(e) = cmd_trigger_remove(&filename) {
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
