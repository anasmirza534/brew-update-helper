use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub fn get_config_path(custom_path: &Option<String>) -> Result<PathBuf> {
    if let Some(path) = custom_path {
        return Ok(PathBuf::from(path));
    }

    // For testing, use current directory
    if std::env::var("CARGO_MANIFEST_DIR").is_ok() {
        return Ok(PathBuf::from("./brew-settings.md"));
    }

    // Production: use ~/.config/brew-update-helper/settings.md
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?
        .join("brew-update-helper");

    Ok(config_dir.join("settings.md"))
}

pub fn read_existing_settings(config_path: &PathBuf) -> Result<HashMap<String, bool>> {
    let mut settings = HashMap::new();

    if !config_path.exists() {
        return Ok(settings);
    }

    let content = fs::read_to_string(config_path)?;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("- [x]") {
            if let Some(package) = line.strip_prefix("- [x] ") {
                settings.insert(package.trim().to_string(), true);
            }
        } else if line.starts_with("- [ ]") {
            if let Some(package) = line.strip_prefix("- [ ] ") {
                settings.insert(package.trim().to_string(), false);
            }
        }
    }

    Ok(settings)
}

pub fn read_previous_packages(config_path: &PathBuf) -> Result<(Vec<String>, Vec<String>)> {
    let mut formulae = Vec::new();
    let mut casks = Vec::new();
    let mut current_section = "";

    if !config_path.exists() {
        return Ok((formulae, casks));
    }

    let content = fs::read_to_string(config_path)?;

    for line in content.lines() {
        let line = line.trim();
        if line == "## Formulae" {
            current_section = "formulae";
        } else if line == "## Casks" {
            current_section = "casks";
        } else if line.starts_with("- [") {
            // Extract package name from checkbox line
            if let Some(package) = extract_package_name(line) {
                match current_section {
                    "formulae" => formulae.push(package),
                    "casks" => casks.push(package),
                    _ => {}
                }
            }
        }
    }

    Ok((formulae, casks))
}

fn extract_package_name(line: &str) -> Option<String> {
    if line.starts_with("- [x] ") {
        line.strip_prefix("- [x] ").map(|s| s.trim().to_string())
    } else if line.starts_with("- [ ] ") {
        line.strip_prefix("- [ ] ").map(|s| s.trim().to_string())
    } else {
        None
    }
}

pub fn generate_settings_content(
    formulae: &[String],
    casks: &[String],
    existing_settings: &HashMap<String, bool>,
    stats: Option<&crate::stats::PackageStats>,
) -> String {
    let mut content = String::new();

    content.push_str("# Brew Auto-Update Settings\n\n");
    content.push_str(&format!(
        "Generated on: {}\n\n",
        Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    ));

    // Add stats section if provided
    if let Some(stats) = stats {
        content.push_str(&stats.format_as_markdown());
    }

    // Formulae section - sort alphabetically
    content.push_str("## Formulae\n\n");
    let mut sorted_formulae = formulae.to_vec();
    sorted_formulae.sort();
    for formula in sorted_formulae {
        let enabled = existing_settings.get(&formula).copied().unwrap_or(true);
        let checkbox = if enabled { "[x]" } else { "[ ]" };
        content.push_str(&format!("- {} {}\n", checkbox, formula));
    }

    // Casks section - sort alphabetically
    content.push_str("\n## Casks\n\n");
    let mut sorted_casks = casks.to_vec();
    sorted_casks.sort();
    for cask in sorted_casks {
        let enabled = existing_settings.get(&cask).copied().unwrap_or(true);
        let checkbox = if enabled { "[x]" } else { "[ ]" };
        content.push_str(&format!("- {} {}\n", checkbox, cask));
    }

    content
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_generate_settings_content() {
        let formulae = vec!["git".to_string(), "node".to_string()];
        let casks = vec!["docker".to_string(), "firefox".to_string()];
        let mut existing_settings = HashMap::new();
        existing_settings.insert("git".to_string(), true);
        existing_settings.insert("node".to_string(), false);
        existing_settings.insert("docker".to_string(), false);

        let content = generate_settings_content(&formulae, &casks, &existing_settings, None);

        assert!(content.contains("# Brew Auto-Update Settings"));
        assert!(content.contains("## Formulae"));
        assert!(content.contains("## Casks"));
        assert!(content.contains("- [x] git"));
        assert!(content.contains("- [ ] node"));
        assert!(content.contains("- [ ] docker"));
        assert!(content.contains("- [x] firefox")); // New package defaults to enabled
    }

    #[test]
    fn test_read_existing_settings() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let settings_path = temp_dir.path().join("settings.md");

        let content = r#"# Brew Auto-Update Settings

Generated on: 2024-08-22 10:30:00 UTC

## Formulae

- [x] git
- [ ] node
- [x] python

## Casks

- [ ] docker
- [x] firefox"#;

        std::fs::write(&settings_path, content)?;

        let settings = read_existing_settings(&settings_path)?;

        assert_eq!(settings.get("git"), Some(&true));
        assert_eq!(settings.get("node"), Some(&false));
        assert_eq!(settings.get("python"), Some(&true));
        assert_eq!(settings.get("docker"), Some(&false));
        assert_eq!(settings.get("firefox"), Some(&true));

        Ok(())
    }

    #[test]
    fn test_get_config_path_development() -> Result<()> {
        // Simulate development environment
        std::env::set_var("CARGO_MANIFEST_DIR", "/some/path");

        let path = get_config_path(&None)?;
        assert_eq!(path, PathBuf::from("./brew-settings.md"));

        std::env::remove_var("CARGO_MANIFEST_DIR");
        Ok(())
    }

    #[test]
    fn test_read_previous_packages() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let settings_path = temp_dir.path().join("settings.md");

        let content = r#"# Brew Auto-Update Settings

Generated on: 2024-08-22 10:30:00 UTC

## Formulae

- [x] git
- [ ] node
- [x] python

## Casks

- [ ] docker
- [x] firefox"#;

        std::fs::write(&settings_path, content)?;

        let (formulae, casks) = read_previous_packages(&settings_path)?;

        assert_eq!(formulae.len(), 3);
        assert!(formulae.contains(&"git".to_string()));
        assert!(formulae.contains(&"node".to_string()));
        assert!(formulae.contains(&"python".to_string()));

        assert_eq!(casks.len(), 2);
        assert!(casks.contains(&"docker".to_string()));
        assert!(casks.contains(&"firefox".to_string()));

        Ok(())
    }

    #[test]
    fn test_extract_package_name() {
        assert_eq!(extract_package_name("- [x] git"), Some("git".to_string()));
        assert_eq!(extract_package_name("- [ ] node"), Some("node".to_string()));
        assert_eq!(extract_package_name("## Formulae"), None);
        assert_eq!(extract_package_name("random text"), None);
    }

    #[test]
    fn test_get_config_path_custom() -> Result<()> {
        let custom_path = Some("/custom/path/settings.md".to_string());
        let path = get_config_path(&custom_path)?;
        assert_eq!(path, PathBuf::from("/custom/path/settings.md"));
        Ok(())
    }
}
