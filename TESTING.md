# Testing Guide

This document describes the testing setup for brew-update-helper.

## Overview

The project includes comprehensive tests that work both on systems with Homebrew installed and in CI/CD environments without Homebrew.

## Test Types

### Unit Tests (src/main.rs)

- **Location**: `src/main.rs` in the `#[cfg(test)] mod tests` section
- **Focus**: Core functions like parsing, settings generation, config path logic
- **Mock Usage**: Uses `MockBrewExecutor` for isolated testing

### Integration Tests (tests/)

- **CLI Tests** (`tests/cli_tests.rs`): Test command-line interface behavior
- **Integration Tests** (`tests/integration_tests.rs`): Test file operations and data parsing

### Test Fixtures (tests/fixtures/)

Sample data files for testing:

- `brew_leaves_output.txt` - Sample formulae list
- `brew_casks_output.txt` - Sample casks list
- `brew_outdated_formulae.txt` - Sample outdated formulae
- `brew_outdated_casks.txt` - Sample outdated casks
- `sample_settings.md` - Sample settings file

## Mock System

### BrewExecutor Trait

The code uses dependency injection with the `BrewExecutor` trait:

- **SystemBrewExecutor**: Real implementation that calls brew commands
- **MockBrewExecutor**: Test implementation with predefined responses

### CI Detection

Automatic environment detection:

```rust
fn create_executor() -> Box<dyn BrewExecutor> {
    #[cfg(test)]
    {
        if std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok() {
            return Box::new(MockBrewExecutor::new());
        }
    }

    Box::new(SystemBrewExecutor)
}
```

## Running Tests

### Local Development

```bash
# Run all tests
cargo test

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test cli_tests
cargo test --test integration_tests

# Run with verbose output
cargo test -- --nocapture
```

### CI Environment

Tests automatically use mocks when `CI` or `GITHUB_ACTIONS` environment variables are set:

```bash
# Simulate CI environment
CI=true cargo test

# Or
GITHUB_ACTIONS=true cargo test
```

## GitHub Actions

The project includes CI configuration (`.github/workflows/ci.yml`) that:

- Runs tests on Ubuntu (without Homebrew)
- Runs tests on macOS (with Homebrew if available)
- Runs tests on Windows for cross-platform compatibility
- Includes formatting and linting checks

## Test Data

### MockBrewExecutor Default Data

```rust
formulae: ["git", "node", "python"]
casks: ["visual-studio-code", "docker", "firefox"]
outdated_packages: [
    OutdatedPackage { name: "git", current: "2.40.0", available: "2.41.0", type: Formula },
    OutdatedPackage { name: "docker", current: "4.18.0", available: "4.19.0", type: Cask }
]
```

### Customizing Test Data

```rust
let executor = MockBrewExecutor::new()
    .with_formulae(vec!["custom-formula".to_string()])
    .with_casks(vec!["custom-cask".to_string()])
    .with_failed_verification(); // Simulate brew not installed
```

## Best Practices

1. **Isolation**: Each test uses temporary directories to avoid affecting the system
2. **Environment Cleanup**: Tests clean up environment variables after running
3. **Cross-Platform**: Tests work on macOS, Linux, and Windows
4. **Mock First**: Always use mocks unless specifically testing system integration
5. **Error Cases**: Test both success and failure scenarios
6. **Terminal Safety**: TUI components are automatically disabled during testing to prevent terminal corruption

## Terminal Cleanup

The project includes automatic terminal state management:

- **TerminalGuard**: RAII pattern ensures terminal state is restored on panic or early return
- **Test Environment Detection**: TUI is automatically disabled when running tests
- **Fallback Behavior**: Uses simple text prompts instead of TUI during testing

This prevents terminal corruption that can occur when TUI tests fail or are interrupted.

## Adding New Tests

When adding new functionality:

1. **Add unit tests** for pure functions in the `tests` module
2. **Add CLI tests** for new command-line behavior
3. **Update MockBrewExecutor** if new brew interactions are added
4. **Add fixtures** for new data formats if needed

## Debugging Tests

```bash
# Run specific test with output
cargo test test_name -- --nocapture

# Run with stack traces
RUST_BACKTRACE=1 cargo test

# Run tests and show ignored tests
cargo test -- --ignored
```
