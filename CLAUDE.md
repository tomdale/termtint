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

- **main.rs** - CLI entry point using clap. Defines commands: `hook`, `apply`, `reset`, `init`, `colors`, `config`
- **config.rs** - Config file discovery (walks up directory tree) and parsing. Supports three formats: simple hex (`#ff5500`), TOML (`tab = "#ff5500"`), and auto-generated colors. Also handles trigger file detection via `ConfigSource` enum
- **user_config.rs** - Global user configuration from `~/.config/termtint/config.toml`. Controls auto color generation parameters, background lightness, and trigger files
- **colors.rs** - Implements the `colors` command for displaying a visual color palette
- **init.rs** - Implements the `init` command for creating `.termtint` files
- **iterm.rs** - Emits iTerm2 OSC escape sequences for tab and background colors
- **state.rs** - Tracks last applied config in `~/.cache/termtint/` to avoid redundant updates. Uses `ConfigSourceType` to distinguish explicit configs from trigger-based auto configs

## Runtime Flow

1. Shell hook (installed via `eval "$(termtint hook zsh)"`) calls `termtint apply` on directory change
2. `apply` searches up from cwd for `.termtint` file OR trigger files (e.g., `Cargo.toml`)
3. Compares against cached state to skip if unchanged
4. Parses config, emits escape sequences, updates state
5. When leaving a termtint directory, resets colors

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

[auto]
hue_min = 0.0
hue_max = 360.0
saturation_min = 0.7
saturation_max = 0.9
lightness = 0.55
```
