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
    /// Apply colors from config
    Apply {
        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,
        /// Force apply even if config is unchanged
        #[arg(short, long)]
        force: bool,
    },
    /// Reset terminal colors to default
    Reset,
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
            if verbose {
                if let Some(source) = &config_source {
                    if let Ok(color_config) = config::parse_config_source(source, &user_config) {
                        eprintln!(
                            "termtint: tab={} {} background={} {} (unchanged)",
                            color_config.tab,
                            color_config.tab.as_color_block(),
                            color_config.background,
                            color_config.background.as_color_block()
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
                            eprintln!(
                                "termtint: tab={} {} background={} {}",
                                color_config.tab,
                                color_config.tab.as_color_block(),
                                color_config.background,
                                color_config.background.as_color_block()
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

fn cmd_reset() {
    iterm::reset_colors();
    state::write_last_config_state(None);
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

    println!("\nCurrent settings:");
    println!(
        "  background_lightness: {:.0}%",
        user_config.background_lightness * 100.0
    );
    if user_config.trigger_files.is_empty() {
        println!("  trigger_files: none");
    } else {
        println!("  trigger_files: {}", user_config.trigger_files.join(", "));
    }

    println!("\nAuto color generation:");
    println!(
        "  hue_range: {:.0}° - {:.0}°",
        user_config.hue_min, user_config.hue_max
    );
    println!(
        "  saturation_range: {:.0}% - {:.0}%",
        user_config.saturation_min * 100.0,
        user_config.saturation_max * 100.0
    );
    println!(
        "  lightness: {:.0}%",
        user_config.lightness * 100.0
    );
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
        Commands::Apply { verbose, force } => {
            cmd_apply(verbose, force);
        }
        Commands::Reset => {
            cmd_reset();
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
    }
}
