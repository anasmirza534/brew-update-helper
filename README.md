# Brew Update Helper

A Rust-based CLI tool that provides selective package upgrade management for Homebrew, allowing users to maintain fine-grained control over which packages get automatically updated.

## Problem

Homebrew users face an all-or-nothing dilemma when upgrading packages:

- `brew upgrade` updates ALL outdated packages, which may be undesirable
- Manual selective upgrades require checking each package individually
- No persistent configuration for upgrade preferences

## Solution

Brew Update Helper maintains a persistent markdown-based configuration of your upgrade preferences and provides an interactive interface for selective package upgrades.

## Features

- **📋 Settings Management**: Generate and maintain package selection preferences in human-readable markdown
- **🎯 Selective Upgrades**: Interactive selection of packages to upgrade with version information
- **📦 Full Support**: Handles both Homebrew formulae and casks
- **👀 Preview Mode**: Dry-run capability to see what would be upgraded
- **📝 Logging**: Comprehensive logging of all upgrade operations
- **⚡ Fallback UI**: Works in both interactive TUI and simple text modes
- **💾 Persistent Settings**: Remembers your preferences across runs

## Installation

### From Source

```bash
git clone https://github.com/anasmirza534/brew-update-helper.git
cd brew-update-helper
cargo build --release
# Binary will be at target/release/brew-update-helper
```

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- [Homebrew](https://brew.sh/) installed and in PATH

## Usage

### 1. Generate Package Settings

First, scan your system and generate a settings file with all manually installed packages:

```bash
brew-update-helper dump
```

This creates a markdown file (default: `~/.config/brew-update-helper/settings.md`) with checkboxes for each package:

```markdown
# Brew Auto-Update Settings

Generated on: 2025-01-15 10:30:00 UTC

## Formulae

- [x] git
- [ ] node
- [x] python@3.11

## Casks

- [x] visual-studio-code
- [ ] docker
- [x] firefox
```

### 2. Customize Your Preferences

Edit the settings file to enable/disable packages:

- `[x]` = Package will be included in upgrades
- `[ ]` = Package will be skipped

### 3. Run Selective Upgrades

Check for outdated packages and upgrade selected ones:

```bash
brew-update-helper upgrade
```

This will:

1. Show all outdated packages that are enabled in your settings
2. Display current → available versions
3. Allow interactive selection (or automatic selection in non-interactive mode)
4. Execute upgrades for selected packages

### Command Line Options

```bash
# Preview what would be done without executing
brew-update-helper --dry-run dump
brew-update-helper --dry-run upgrade

# Use custom settings file location
brew-update-helper --config ./my-settings.md dump
brew-update-helper --config ./my-settings.md upgrade

# Get help
brew-update-helper --help
brew-update-helper dump --help
brew-update-helper upgrade --help
```

## Interactive Interface

When running `upgrade`, you'll see an interactive interface:

```
┌─ Outdated packages found - Select packages to upgrade ─┐
│                                                        │
└────────────────────────────────────────────────────────┘
┌────────────────────────────────────────────────────────┐
│ [x] git (Formula) 2.40.0 → 2.41.0                     │
│ [ ] node (Formula) 18.16.0 → 20.5.0                   │
│ [x] visual-studio-code (Cask) 1.80.0 → 1.81.0         │
└────────────────────────────────────────────────────────┘
┌─ ↑↓: Navigate, SPACE: Toggle, ENTER: Proceed, q: Quit ─┐
└────────────────────────────────────────────────────────┘
```

**Controls:**

- `↑↓` - Navigate between packages
- `SPACE` - Toggle package selection
- `ENTER` - Proceed with upgrade
- `q` - Quit without upgrading

## Configuration

### Settings File Locations

- **Production**: `~/.config/brew-update-helper/settings.md`
- **Development**: `./brew-settings.md` (when running from source)

### Log Files

Upgrade operations are logged to:

- **Production**: `~/.config/brew-update-helper/upgrade.log`
- **Development**: `./brew-update-helper.log`

### Custom Configuration

Use the `--config` flag to specify a custom settings file path:

```bash
brew-update-helper --config /path/to/my-settings.md dump
```

## Examples

### Basic Workflow

```bash
# 1. Generate initial settings
brew-update-helper dump

# 2. Edit settings file to customize preferences
vim ~/.config/brew-update-helper/settings.md

# 3. Preview what would be upgraded
brew-update-helper --dry-run upgrade

# 4. Run actual upgrades
brew-update-helper upgrade
```

### Automation-Friendly Usage

```bash
# Non-interactive mode with simple y/n prompt
echo "y" | brew-update-helper upgrade

# Dry-run for CI/scripts
brew-update-helper --dry-run upgrade
```

## Development

### Building

```bash
cargo build
```

### Running Tests

```bash
cargo test
```

### Running from Source

```bash
cargo run -- dump
cargo run -- --dry-run upgrade
```

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [Rust](https://www.rust-lang.org/)
- CLI framework: [clap](https://github.com/clap-rs/clap)
- TUI framework: [ratatui](https://github.com/ratatui-org/ratatui)
- Terminal handling: [crossterm](https://github.com/crossterm-rs/crossterm)
- Integrates with [Homebrew](https://brew.sh/)
- Developed with assistance from [Anthropic Claude Code](https://claude.ai/code)
