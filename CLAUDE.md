# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Brew Update Helper is a Rust CLI tool for selective Homebrew package upgrade management. It allows users to maintain fine-grained control over which packages get automatically updated, addressing the all-or-nothing dilemma of `brew upgrade`.

## Core Architecture

The application is built as a single-file Rust binary (`src/main.rs`) with two main subcommands:

### Commands Structure

- **`dump`** - Generates/updates package selection settings by scanning manually installed formulae and casks
- **`upgrade`** - Performs selective upgrades based on user settings with interactive TUI selection

### Key Components

**Settings Management** (`dump_command`):

- Scans manually installed packages using `brew leaves --installed-on-request` (formulae) and `brew list --cask` (casks)
- Generates markdown settings file with checkbox format for user preference management
- Preserves existing user selections when regenerating settings
- Uses environment detection for config paths (development vs production)

**Upgrade Logic** (`upgrade_command`):

- Parses markdown settings to determine enabled packages
- Queries outdated packages via `brew outdated --verbose` for both formulae and casks
- Provides interactive TUI selection using ratatui/crossterm with fallback to simple text prompts
- Executes upgrades individually with comprehensive error handling and logging

**Package Detection**:

- `OutdatedPackage` struct tracks name, versions, and type (Formula/Cask)
- Parses brew's verbose output format: "package (current_version) < available_version"
- Handles both formula and cask upgrade commands with appropriate flags

## Common Development Commands

```bash
# Build the project
cargo build

# Run from source
cargo run -- dump
cargo run -- --dry-run upgrade
cargo run -- --config custom-path.md dump

# Run tests
cargo test

# Build release binary
cargo build --release
# Binary will be at target/release/brew-update-helper
```

## Configuration Behavior

**Settings File Locations**:

- Development (when `CARGO_MANIFEST_DIR` is set): `./brew-settings.md`
- Production: `~/.config/brew-update-helper/settings.md`

**Log File Locations**:

- Development: `./brew-update-helper.log`
- Production: `~/.config/brew-update-helper/upgrade.log`

**Settings File Format**:
The tool generates human-readable markdown with checkboxes:

```markdown
# Brew Auto-Update Settings

Generated on: YYYY-MM-DD HH:MM:SS UTC

## Formulae

- [x] git
- [ ] node

## Casks

- [x] visual-studio-code
- [ ] docker
```

## Key Dependencies

- **clap** - CLI argument parsing with derive macros
- **ratatui + crossterm** - Terminal UI for interactive package selection
- **anyhow** - Error handling throughout the application
- **chrono** - Timestamp generation for settings and logs
- **dirs** - Cross-platform config directory detection

## Error Handling Patterns

The codebase uses `anyhow::Result<()>` consistently and includes:

- Brew installation verification before any operations
- Individual package upgrade failure handling (continues with remaining packages)
- Graceful TUI fallback to simple text prompts
- Comprehensive logging of all operations with timestamps

## Testing Notes

The application includes development/production environment detection via `CARGO_MANIFEST_DIR` environment variable, allowing for local testing without affecting system configuration files.
