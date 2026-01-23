# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**termtint** is a Rust CLI tool that automatically colorizes iTerm2 terminal tabs and backgrounds based on `.termtint` configuration files found in directories. When you `cd` into a directory with a config file (or with one in a parent directory), the terminal applies custom colors via shell hooks.

## Build & Test Commands

```bash
cargo build              # Debug build
cargo build --release    # Release build
cargo test               # Run all tests
cargo test <test_name>   # Run a single test
```

## Architecture

The codebase has seven modules:

- **main.rs** - CLI entry point using clap. Defines commands: `hook`, `apply`, `reset`, `init`, `reroll`, `colors`, `config`, `inspect`, `trigger`
- **config.rs** - Config file discovery (walks up directory tree) and parsing. Supports three formats: simple hex (`#ff5500`), TOML (`tab = "#ff5500"`), and auto-generated colors. Also handles trigger file detection via `ConfigSource` enum. Key public functions:
  - `parse_color()` - Parse color strings (hex, RGB, HSL, named colors)
  - `find_config_source()` - Walk up directory tree to find config or trigger files
  - `parse_config_source()` - Parse a ConfigSource into ColorConfig
  - `detect_format()` - Determine config file format
  - `generate_random_color()` - Generate random color using user config parameters
- **user_config.rs** - Global user configuration from `~/.config/termtint/config.toml`. Controls auto color generation parameters, background lightness, trigger files, and color display format. Supports three `ColorFormat` options: Hex, HSL, RGB. Key public functions:
  - `load_user_config()` - Load config from file or return defaults
  - `save_trigger_files()` - Update trigger_files in config file
  - `config_file_path()` - Get path to config file
  - `default_config_toml()` - Generate default config template
- **colors.rs** - Implements the `colors` command for displaying a visual color palette with a 2D saturation grid (hue on X-axis, saturation on Y-axis) and sample tab/background pairs
- **init.rs** - Implements the `init` and `reroll` commands for creating and re-rolling `.termtint` files. The `reroll` command displays ASCII dice art using the new colors
- **iterm.rs** - Emits iTerm2 OSC escape sequences for tab and background colors. Key public functions:
  - `apply_colors()` - Apply tab and background colors
  - `reset_colors()` - Reset to default colors
  - `get_reset_sequences()` - Get escape sequences for verbose output
- **state.rs** - Tracks last applied config in `~/.cache/termtint/` to avoid redundant updates. Uses `ConfigSourceType` to distinguish explicit configs from trigger-based auto configs. Includes `cleanup_stale_sessions()` to remove old state files

## Runtime Flow

1. Shell hook (installed via `eval "$(termtint hook zsh)"`) calls `termtint apply` on directory change
2. `apply` searches up from cwd for `.termtint` file OR trigger files (e.g., `Cargo.toml`)
3. Compares against cached state to skip if unchanged
4. Parses config, emits escape sequences, updates state
5. When leaving a termtint directory, resets colors

## Commands

- **hook** - Print shell integration code for zsh
- **apply** - Apply colors from config (supports `--verbose`, `--force`, `--info` flags)
- **reset** - Reset terminal colors to default (supports `--verbose` flag)
- **init** - Create a `.termtint` file (supports optional color, `--background`, `--force`)
- **reroll** - Re-roll to a new random color, shows ASCII dice art (supports `--force`)
- **colors** - Display visual color palette with 2D saturation grid and sample pairs
- **config** - Show current configuration (supports `--edit`, `--path` flags)
- **inspect** - Inspect current directory's color configuration, showing source, resolved colors, and cached state
- **trigger** - Manage trigger files (subcommands: `add`, `remove`, `list`)

## Config Formats

Simple hex (background auto-darkened):
```
#ff5500
```

TOML (explicit control):
```toml
tab = "#00ff00"
background = "#001100"
```

Auto (hash-based deterministic color):
```
auto
```

## User Config

Global settings in `~/.config/termtint/config.toml`:
```toml
background_lightness = 0.10
trigger_files = ["Cargo.toml", "package.json"]
color_format = "hex"  # Options: "hex", "hsl", "rgb"

[auto]
hue_min = 0.0
hue_max = 360.0
saturation_min = 0.7
saturation_max = 0.9
lightness = 0.55
```

## Command Flags

### apply command
- `--verbose` / `-v` - Show detailed output with color swatches
- `--force` / `-f` - Force apply even if config is unchanged
- `--info` - Show detailed config information (source type, format, raw config, resolved colors)

### reset command
- `--verbose` / `-v` - Show escape sequences being emitted, state file info, and previous state

### inspect command
No flags. Shows current directory's config source (explicit `.termtint` or trigger file), matched trigger file if applicable, resolved colors with color blocks, and cached state information.

### trigger command
Subcommands for managing trigger files:
- `trigger add <filename>` - Add a file to the trigger list
- `trigger remove <filename>` - Remove a file from the trigger list
- `trigger list` - List all configured trigger files

## Key Features

### Color Display Formats
Colors can be displayed in three formats (configured via `color_format` in user config):
- **hex** - `#ff5500` (default)
- **hsl** - `hsl(20, 100%, 50%)`
- **rgb** - `rgb(255, 85, 0)`

### Enhanced Color Palette
The `termtint colors` command displays:
- 2D saturation grid with hue on X-axis (36 steps) and saturation on Y-axis (100%, 80%, 60%, 40%)
- 12 sample tab/background color pairs showing the configured lightness and background darkening

### Dice Animation
The `termtint reroll` command shows ASCII dice art using the new tab and background colors, with a random die face (1-6) rendered with Unicode box-drawing characters.
