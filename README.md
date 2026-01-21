# termtint

Terminal color theming based on directory - colorizes iTerm2 tabs and backgrounds based on `.termtint` config files.

## Installation

```bash
cargo install termtint
```

Or build from source:

```bash
cargo build --release
cp target/release/termtint ~/.local/bin/  # or somewhere in your PATH
```

Add to your `.zshrc`:

```zsh
eval "$(termtint hook zsh)"
```

## Usage

Create a `.termtint` file in any directory. When you `cd` into that directory (or any subdirectory), terminal colors will automatically change.

### Config Formats

**Hex color** (simplest):
```
#ff5500
```

**TOML** (more control):
```toml
tab = "#00ff00"
background = "#001100"  # optional, defaults to 15% of tab color
```

**Auto** (deterministic hash-based color):
```
auto
```

### Commands

```bash
termtint hook zsh   # Output shell hook (add to .zshrc)
termtint apply      # Apply colors for current directory
termtint reset      # Reset colors to defaults
```

## How It Works

1. Shell hook calls `termtint apply` on every directory change
2. `apply` searches up from current directory for `.termtint`
3. If found, parses config and emits iTerm2 escape sequences
4. State is tracked in `~/.cache/termtint/` to avoid redundant updates
