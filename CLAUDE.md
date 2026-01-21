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

The codebase has four modules:

- **main.rs** - CLI entry point using clap. Defines commands: `hook`, `apply`, `reset`
- **config.rs** - Config file discovery (walks up directory tree) and parsing. Supports three formats: simple hex (`#ff5500`), TOML (`tab = "#ff5500"`), and auto-generated colors
- **iterm.rs** - Emits iTerm2 OSC escape sequences for tab and background colors
- **state.rs** - Tracks last applied config in `~/.cache/termtint/last_config` to avoid redundant updates

## Runtime Flow

1. Shell hook (installed via `eval "$(termtint hook zsh)"`) calls `termtint apply` on directory change
2. `apply` searches up from cwd for `.termtint` file
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
