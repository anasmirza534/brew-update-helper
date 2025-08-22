use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_cli_dump_command() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("settings.md");

    // Mock environment for testing
    std::env::set_var("CI", "true");

    let mut cmd = Command::cargo_bin("brew-update-helper").unwrap();
    cmd.arg("--config")
        .arg(config_path.to_string_lossy().to_string())
        .arg("dump")
        .assert()
        .success()
        .stdout(predicate::str::contains("Found"))
        .stdout(predicate::str::contains("manually installed"));

    // Cleanup
    std::env::remove_var("CI");
}

#[test]
fn test_cli_dump_dry_run() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("settings.md");

    // Mock environment for testing
    std::env::set_var("CI", "true");

    let mut cmd = Command::cargo_bin("brew-update-helper").unwrap();
    cmd.arg("--dry-run")
        .arg("--config")
        .arg(config_path.to_string_lossy().to_string())
        .arg("dump")
        .assert()
        .success()
        .stdout(predicate::str::contains("(dry run mode)"))
        .stdout(predicate::str::contains("Would write settings to:"));

    // Cleanup
    std::env::remove_var("CI");
}

#[test]
fn test_cli_upgrade_no_settings() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("nonexistent.md");

    // Mock environment for testing
    std::env::set_var("CI", "true");

    let mut cmd = Command::cargo_bin("brew-update-helper").unwrap();
    cmd.arg("--config")
        .arg(config_path.to_string_lossy().to_string())
        .arg("upgrade")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Settings file not found"));

    // Cleanup
    std::env::remove_var("CI");
}

#[test]
fn test_cli_upgrade_with_settings() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("settings.md");

    // Create a sample settings file
    let settings_content = r#"# Brew Auto-Update Settings

Generated on: 2024-08-22 10:30:00 UTC

## Formulae

- [x] git
- [ ] node

## Casks

- [x] docker
- [ ] firefox"#;

    fs::write(&config_path, settings_content).unwrap();

    // Mock environment for testing
    std::env::set_var("CI", "true");

    let mut cmd = Command::cargo_bin("brew-update-helper").unwrap();
    cmd.arg("--dry-run")
        .arg("--config")
        .arg(config_path.to_string_lossy().to_string())
        .arg("upgrade")
        .assert()
        .success()
        .stdout(predicate::str::contains("Checking for outdated packages"));

    // Cleanup
    std::env::remove_var("CI");
}

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("brew-update-helper").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "selective Homebrew package upgrade management",
        ));
}

#[test]
fn test_cli_version() {
    let mut cmd = Command::cargo_bin("brew-update-helper").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("brew-update-helper"));
}

#[test]
fn test_cli_invalid_command() {
    let mut cmd = Command::cargo_bin("brew-update-helper").unwrap();
    cmd.arg("invalid-command").assert().failure();
}
