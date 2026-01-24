# termtint

![termtint hero image showing four terminal windows with different color themes](assets/hero.png)

Automatically and deterministically colorizes iTerm2 tabs and backgrounds as you
cd between directories.

## Installation

```bash
cargo install termtint
```

Or build from source:

```bash
cargo build --release
cp target/release/termtint ~/.local/bin/  # or somewhere in your PATH
```

Add the shell hook to your shell config:

**Zsh** (`~/.zshrc`):

```zsh
eval "$(termtint hook zsh)"
```

**Bash** (`~/.bashrc`):

```bash
eval "$(termtint hook bash)"
```

**Fish** (`~/.config/fish/config.fish`):

```fish
termtint hook fish | source
```

## Usage

Create a `.termtint` file in any directory. When you `cd` into that directory
(or any subdirectory), terminal colors will automatically change.

> **Note:** termtint uses iTerm2-specific escape sequences for setting tab and
> background colors. These are non-standard extensions and will only work in
> iTerm2 on macOS.

### .termtint

The `.termtint` file is a simple text file that specifies the color of the tab
and background for that directory.

**Hex color**

When `.termtint` contains a simple color value, it will be used as the tab
color, and the background color will be automatically calculated.

```
#ff5500
```

Colors can be specified in multiple formats:

- 6-digit hex: `#ff5500` or `ff5500`
- 3-digit hex: `#f50`
- RGB: `rgb(255, 85, 0)`
- HSL: `hsl(20, 100%, 50%)`
- Named colors: `red`, `tomato`, `dodgerblue`, etc.

**TOML**

```toml
tab = "#00ff00"
background = "#001100" # optional, will be calculated if omitted
```

**Auto**

When `.termtint` contains `auto`, the tab and background colors will be selected
using a deterministic hash of the directory path.

```
auto
```

### Automatic Colorization

You can configure termtint to automatically colorize directories without needing
to create `.termtint` files. Colors are deterministically generated from the
directory path, so each directory always gets the same color.

**Trigger files** automatically apply colors when entering directories with
specific project files:

```bash
termtint trigger add Cargo.toml     # Colorize Rust projects
termtint trigger add package.json   # Colorize Node.js projects
termtint trigger add go.mod         # Colorize Go projects
```

**Trigger paths** automatically apply colors to directories matching glob
patterns:

```bash
termtint trigger add "~/Code/*"      # Colorize all directories in ~/Code
termtint trigger add "~/Projects/*"  # Colorize all directories in ~/Projects
```

The `trigger add` command automatically detects whether you're adding a file
name or a path pattern. Patterns containing `/`, `*`, `~`, or `?` are treated as
paths.

Manage triggers:

```bash
termtint trigger list               # List all triggers
termtint trigger remove Cargo.toml  # Remove a trigger
```

### Commands

```bash
termtint hook <shell>    # Output shell hook (zsh, bash, or fish)
termtint apply           # Apply colors for current directory
termtint apply --verbose # Show detailed config info and color swatches
termtint apply --force   # Force apply even if config is unchanged
termtint reset           # Reset colors to defaults
termtint reset --verbose # Show escape sequences and state file info
termtint init            # Create .termtint with auto color
termtint init '#ff5500'  # Create .termtint with specific color
termtint init 'green'    # Named colors are normalized to hex (#008000)
termtint init '#ff5500' --background '#1a0800'  # With custom background
termtint init --force    # Overwrite existing .termtint
termtint reroll          # Re-roll to a new random color (creates .termtint if needed)
termtint reroll --verbose # Show directory path
termtint colors          # Display color palette and configuration
termtint config          # Show current configuration settings
termtint config --edit   # Open config file in $EDITOR
termtint config --path   # Print config file path
termtint inspect         # Show current directory's config source and colors
termtint trigger list    # List all triggers
termtint trigger add <pattern>     # Add a trigger (file or path)
termtint trigger remove <pattern>  # Remove a trigger
```

## How It Works

1. Shell hook calls `termtint apply` on every directory change
2. `apply` searches up from current directory for `.termtint` or trigger matches
3. If found, parses config and emits iTerm2 escape sequences
4. State is tracked in `~/.cache/termtint/` to avoid redundant updates

## Advanced Features

### Verbose Output

Use `--verbose` with `apply` or `reset` to see detailed output:

```bash
termtint apply --verbose  # Shows config info and color swatches
termtint reset --verbose  # Shows escape sequences and state file info
```

The verbose output for `apply` displays:

- Config source type (explicit `.termtint` file, trigger path, or trigger file)
- Source path and format (auto, simple hex, or TOML)
- Whether background is explicit or auto-generated
- Resolved RGB colors with large color swatches

### Inspect Current Directory

The `inspect` command shows the current directory's configuration:

```bash
termtint inspect
```

Output includes:

- Current directory path
- Config source (`.termtint` file, trigger path, trigger file, or none)
- Matched pattern or trigger file (if applicable)
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

- Updates `.termtint` with a new random color (creates it if it doesn't exist)
- Shows ASCII dice art with the new colors
- Applies colors immediately

### Configuration

User configuration is stored in `~/.config/termtint/config.toml`:

```toml
# Fixed lightness for darkened backgrounds (0.0 to 1.0)
background_lightness = 0.18

# Saturation multiplier for backgrounds (0.0 = grayscale, 1.0 = full color)
background_saturation = 1.0

# Files that trigger automatic color generation when found
trigger_files = ["Cargo.toml", "package.json"]

# Path globs that trigger automatic color generation
# Directories matching these patterns get auto-generated colors
trigger_paths = ["~/Code/*", "~/Projects/*"]

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
termtint config -e
```

View current settings:

```bash
termtint config
```

### Color Formats

Colors can be specified in multiple formats in `.termtint` files:

- 6-digit hex: `#ff5500` or `ff5500`
- 3-digit hex: `#f50`
- RGB: `rgb(255, 85, 0)`
- HSL: `hsl(20, 100%, 50%)`
- Named colors: `red`, `tomato`, `dodgerblue`, etc.

When using `termtint init`, all color formats are validated and normalized to
hex format (e.g., `green` becomes `#008000`).
