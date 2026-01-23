# termtint

![termtint hero image showing four terminal windows with different color themes](assets/hero.png)

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

Colors can be specified in multiple formats:
- 6-digit hex: `#ff5500` or `ff5500`
- 3-digit hex: `#f50`
- RGB: `rgb(255, 85, 0)`
- HSL: `hsl(20, 100%, 50%)`
- Named colors: `red`, `tomato`, `dodgerblue`, etc.

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
termtint hook zsh       # Output shell hook (add to .zshrc)
termtint apply          # Apply colors for current directory
termtint apply --verbose   # Show color swatches when applying
termtint apply --info   # Show detailed config information before applying
termtint reset          # Reset colors to defaults
termtint reset --verbose   # Show escape sequences and state file info
termtint init           # Create .termtint with auto color
termtint init '#ff5500' # Create .termtint with specific color
termtint init 'green'   # Named colors are normalized to hex (#008000)
termtint init '#ff5500' --background '#1a0800'  # With custom background
termtint init -f        # Overwrite existing .termtint
termtint reroll         # Re-roll to a new random color (updates .termtint)
termtint reroll -f      # Create .termtint with random color if it doesn't exist
termtint colors         # Display color palette and configuration
termtint config         # Show current configuration settings
termtint config --edit  # Open config file in $EDITOR
termtint config --path  # Print config file path
termtint inspect        # Show current directory's config source and colors
termtint trigger list   # List configured trigger files
termtint trigger add Cargo.toml    # Add a trigger file
termtint trigger remove Cargo.toml # Remove a trigger file
```

## How It Works

1. Shell hook calls `termtint apply` on every directory change
2. `apply` searches up from current directory for `.termtint`
3. If found, parses config and emits iTerm2 escape sequences
4. State is tracked in `~/.cache/termtint/` to avoid redundant updates

## Advanced Features

### Verbose Output

Use `--verbose` with `apply` or `reset` to see detailed output:

```bash
termtint apply --verbose   # Shows large color swatches (6 lines tall)
termtint reset --verbose   # Shows escape sequences and state file info
```

### Detailed Config Info

Use `--info` with `apply` to see comprehensive config information:

```bash
termtint apply --info
```

This displays:
- Config source type (explicit `.termtint` file or trigger-based)
- Source path and format (auto, simple hex, or TOML)
- Raw config file contents
- Whether background is explicit or auto-generated
- Resolved RGB colors

### Inspect Current Directory

The `inspect` command shows the current directory's configuration:

```bash
termtint inspect
```

Output includes:
- Current directory path
- Config source (`.termtint` file, trigger file, or none)
- Matched trigger file (if applicable)
- Resolved tab and background colors with color blocks
- Cached state information

### Color Palette

The `colors` command displays a visual palette of available colors:

```bash
termtint colors
```

Features:
- Shows a 2D grid with hue on X-axis and saturation on Y-axis
- Displays current configuration parameters
- Shows sample tab/background color pairs
- Uses your configured color format (hex, HSL, or RGB)

### Re-roll Colors

Generate a new random color for the current directory:

```bash
termtint reroll
```

Features:
- Updates `.termtint` with a new random color
- Shows ASCII dice art with the new colors
- Applies colors immediately
- Use `--force` to create `.termtint` if it doesn't exist

### Configuration

User configuration is stored in `~/.config/termtint/config.toml`:

```toml
# Fixed lightness for darkened backgrounds (0.0 to 1.0)
background_lightness = 0.10

# Files that trigger automatic color generation when found
trigger_files = ["Cargo.toml", "package.json"]

# Color format for display: "hex", "hsl", or "rgb"
color_format = "hex"

# Auto color generation parameters
[auto]
hue_min = 0.0
hue_max = 360.0
saturation_min = 0.7
saturation_max = 0.9
lightness = 0.55
```

Edit the config:
```bash
termtint config --edit
```

View current settings:
```bash
termtint config
```

### Trigger Files

Configure trigger files to automatically apply colors when entering directories with specific project files (e.g., `Cargo.toml`, `package.json`). Colors are deterministically generated from the directory path.

Manage trigger files from the command line:
```bash
termtint trigger list              # List current trigger files
termtint trigger add Cargo.toml    # Add a trigger file
termtint trigger add package.json  # Add another
termtint trigger remove Cargo.toml # Remove a trigger file
```

### Color Formats

Colors can be specified in multiple formats in `.termtint` files:
- 6-digit hex: `#ff5500` or `ff5500`
- 3-digit hex: `#f50`
- RGB: `rgb(255, 85, 0)`
- HSL: `hsl(20, 100%, 50%)`
- Named colors: `red`, `tomato`, `dodgerblue`, etc.

When using `termtint init`, all color formats are validated and normalized to hex format (e.g., `green` becomes `#008000`).
