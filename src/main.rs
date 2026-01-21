use clap::{Parser, Subcommand};

mod config;
mod init;
mod iterm;
mod state;

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
}

fn cmd_apply(verbose: bool) {
    state::cleanup_stale_sessions();

    let current_dir = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("Error getting current directory: {}", e);
            return;
        }
    };

    let config_path = config::find_config(&current_dir);
    let last_state = state::read_last_config_state();

    // Build current state if we have a config
    let current_state = config_path.as_ref().and_then(|path| {
        state::get_file_mtime(path).map(|mtime| state::ConfigState {
            path: path.clone(),
            mtime,
        })
    });

    match (&current_state, &last_state) {
        // Same config file and unchanged, no change needed
        (Some(current), Some(last)) if current == last => {
            if verbose {
                if let Ok(color_config) = config::parse_config(&current.path) {
                    eprintln!("termtint: tab={} background={} (unchanged)", color_config.tab, color_config.background);
                }
            }
        }

        // Found a config file (new or changed)
        (Some(current), _) => {
            match config::parse_config(&current.path) {
                Ok(color_config) => {
                    if verbose {
                        eprintln!("termtint: tab={} background={}", color_config.tab, color_config.background);
                    }
                    iterm::apply_colors(&color_config);
                    state::write_last_config_state(Some(current));
                }
                Err(e) => {
                    eprintln!("Error parsing config: {}", e);
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

        // No config found and none before - nothing to do
        (None, None) => {
            if verbose {
                eprintln!("termtint: no config found");
            }
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

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Hook { shell } => {
            cmd_hook(&shell);
        }
        Commands::Apply { verbose } => {
            cmd_apply(verbose);
        }
        Commands::Reset => {
            cmd_reset();
        }
        Commands::Init {
            color,
            background,
            force,
        } => {
            if let Err(e) = init::cmd_init(color, background, force) {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
    }
}
