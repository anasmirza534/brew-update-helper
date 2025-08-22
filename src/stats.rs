use anyhow::Result;
use std::collections::HashMap;
use std::process::Command;

use crate::brew::{BrewExecutor, PackageType};

#[derive(Debug, Clone)]
pub struct PackageStats {
    pub total_formulae: usize,
    pub total_casks: usize,
    pub total_packages: usize,
    pub enabled_formulae: usize,
    pub enabled_casks: usize,
    pub disabled_formulae: usize,
    pub disabled_casks: usize,
    pub outdated_formulae: usize,
    pub outdated_casks: usize,
    pub total_outdated: usize,
    pub homebrew_version: String,
    pub system_info: SystemInfo,
    pub changes: PackageChanges,
}

#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub os_version: String,
    pub architecture: String,
    pub homebrew_prefix: String,
}

#[derive(Debug, Clone)]
pub struct PackageChanges {
    pub added_formulae: usize,
    pub removed_formulae: usize,
    pub added_casks: usize,
    pub removed_casks: usize,
}

impl PackageStats {
    pub fn collect(
        executor: &dyn BrewExecutor,
        current_formulae: &[String],
        current_casks: &[String],
        existing_settings: &HashMap<String, bool>,
        previous_formulae: Option<&[String]>,
        previous_casks: Option<&[String]>,
    ) -> Result<Self> {
        let total_formulae = current_formulae.len();
        let total_casks = current_casks.len();
        let total_packages = total_formulae + total_casks;

        // Count enabled/disabled packages from existing settings
        let (enabled_formulae, disabled_formulae) =
            count_enabled_disabled(current_formulae, existing_settings);
        let (enabled_casks, disabled_casks) =
            count_enabled_disabled(current_casks, existing_settings);

        // Get outdated package counts
        let outdated_packages = executor.get_outdated_packages().unwrap_or_default();
        let outdated_formulae = outdated_packages
            .iter()
            .filter(|pkg| matches!(pkg.package_type, PackageType::Formula))
            .count();
        let outdated_casks = outdated_packages
            .iter()
            .filter(|pkg| matches!(pkg.package_type, PackageType::Cask))
            .count();
        let total_outdated = outdated_formulae + outdated_casks;

        // Collect system information
        let homebrew_version = get_homebrew_version()?;
        let system_info = collect_system_info()?;

        // Calculate package changes
        let changes = calculate_package_changes(
            current_formulae,
            current_casks,
            previous_formulae,
            previous_casks,
        );

        Ok(PackageStats {
            total_formulae,
            total_casks,
            total_packages,
            enabled_formulae,
            enabled_casks,
            disabled_formulae,
            disabled_casks,
            outdated_formulae,
            outdated_casks,
            total_outdated,
            homebrew_version,
            system_info,
            changes,
        })
    }

    pub fn format_as_markdown(&self) -> String {
        let mut content = String::new();

        content.push_str("## Statistics\n\n");

        // Basic package counts
        content.push_str(&format!(
            "- **Total Packages**: {} ({} formulae, {} casks)\n",
            self.total_packages, self.total_formulae, self.total_casks
        ));

        // Enabled/disabled breakdown
        if self.enabled_formulae + self.enabled_casks > 0 {
            content.push_str(&format!(
                "- **Enabled for Auto-Update**: {} ({} formulae, {} casks)\n",
                self.enabled_formulae + self.enabled_casks,
                self.enabled_formulae,
                self.enabled_casks
            ));
        }

        if self.disabled_formulae + self.disabled_casks > 0 {
            content.push_str(&format!(
                "- **Disabled for Auto-Update**: {} ({} formulae, {} casks)\n",
                self.disabled_formulae + self.disabled_casks,
                self.disabled_formulae,
                self.disabled_casks
            ));
        }

        // Outdated packages
        if self.total_outdated > 0 {
            content.push_str(&format!(
                "- **Outdated Packages**: {} ({} formulae, {} casks)\n",
                self.total_outdated, self.outdated_formulae, self.outdated_casks
            ));
        } else {
            content.push_str("- **Outdated Packages**: All packages up to date! 🎉\n");
        }

        // System information
        content.push_str(&format!(
            "- **Homebrew Version**: {}\n",
            self.homebrew_version
        ));
        content.push_str(&format!("- **System**: {}\n", self.system_info.os_version));
        content.push_str(&format!(
            "- **Architecture**: {}\n",
            self.system_info.architecture
        ));
        content.push_str(&format!(
            "- **Homebrew Prefix**: {}\n",
            self.system_info.homebrew_prefix
        ));

        // Package changes
        if self.changes.has_changes() {
            content.push_str("- **Changes Since Last Dump**:");
            if self.changes.added_formulae > 0 {
                content.push_str(&format!(" +{} formulae", self.changes.added_formulae));
            }
            if self.changes.removed_formulae > 0 {
                content.push_str(&format!(" -{} formulae", self.changes.removed_formulae));
            }
            if self.changes.added_casks > 0 {
                content.push_str(&format!(" +{} casks", self.changes.added_casks));
            }
            if self.changes.removed_casks > 0 {
                content.push_str(&format!(" -{} casks", self.changes.removed_casks));
            }
            content.push('\n');
        }

        content.push('\n');
        content
    }
}

impl PackageChanges {
    pub fn has_changes(&self) -> bool {
        self.added_formulae > 0
            || self.removed_formulae > 0
            || self.added_casks > 0
            || self.removed_casks > 0
    }
}

fn count_enabled_disabled(packages: &[String], settings: &HashMap<String, bool>) -> (usize, usize) {
    let mut enabled = 0;
    let mut disabled = 0;

    for package in packages {
        if settings.get(package).copied().unwrap_or(true) {
            enabled += 1;
        } else {
            disabled += 1;
        }
    }

    (enabled, disabled)
}

fn get_homebrew_version() -> Result<String> {
    let output = Command::new("brew").arg("--version").output()?;

    if !output.status.success() {
        return Ok("Unknown".to_string());
    }

    let version_output = String::from_utf8(output.stdout)?;
    // Extract first line which contains the version
    let version = version_output
        .lines()
        .next()
        .unwrap_or("Unknown")
        .trim()
        .to_string();

    Ok(version)
}

fn collect_system_info() -> Result<SystemInfo> {
    // Get Homebrew prefix
    let homebrew_prefix = get_homebrew_prefix()?;

    // Get system information
    let os_version = get_os_version();
    let architecture = get_architecture();

    Ok(SystemInfo {
        os_version,
        architecture,
        homebrew_prefix,
    })
}

fn get_homebrew_prefix() -> Result<String> {
    let output = Command::new("brew").arg("--prefix").output()?;

    if !output.status.success() {
        return Ok("Unknown".to_string());
    }

    let prefix = String::from_utf8(output.stdout)?.trim().to_string();

    Ok(prefix)
}

fn get_os_version() -> String {
    let output = Command::new("sw_vers").arg("-productVersion").output();

    match output {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            format!("macOS {}", version)
        }
        _ => "Unknown OS".to_string(),
    }
}

fn get_architecture() -> String {
    let output = Command::new("uname").arg("-m").output();

    match output {
        Ok(output) if output.status.success() => {
            let arch_output = String::from_utf8_lossy(&output.stdout);
            let arch = arch_output.trim();
            match arch {
                "arm64" => "Apple Silicon".to_string(),
                "x86_64" => "Intel".to_string(),
                _ => arch.to_string(),
            }
        }
        _ => "Unknown".to_string(),
    }
}

fn calculate_package_changes(
    current_formulae: &[String],
    current_casks: &[String],
    previous_formulae: Option<&[String]>,
    previous_casks: Option<&[String]>,
) -> PackageChanges {
    let mut changes = PackageChanges {
        added_formulae: 0,
        removed_formulae: 0,
        added_casks: 0,
        removed_casks: 0,
    };

    if let Some(prev_formulae) = previous_formulae {
        changes.added_formulae = current_formulae
            .iter()
            .filter(|pkg| !prev_formulae.contains(pkg))
            .count();
        changes.removed_formulae = prev_formulae
            .iter()
            .filter(|pkg| !current_formulae.contains(pkg))
            .count();
    }

    if let Some(prev_casks) = previous_casks {
        changes.added_casks = current_casks
            .iter()
            .filter(|pkg| !prev_casks.contains(pkg))
            .count();
        changes.removed_casks = prev_casks
            .iter()
            .filter(|pkg| !current_casks.contains(pkg))
            .count();
    }

    changes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brew::MockBrewExecutor;

    #[test]
    fn test_count_enabled_disabled() {
        let packages = vec!["git".to_string(), "node".to_string(), "python".to_string()];
        let mut settings = HashMap::new();
        settings.insert("git".to_string(), true);
        settings.insert("node".to_string(), false);
        // python not in settings, should default to true

        let (enabled, disabled) = count_enabled_disabled(&packages, &settings);
        assert_eq!(enabled, 2); // git and python
        assert_eq!(disabled, 1); // node
    }

    #[test]
    fn test_calculate_package_changes() {
        let current_formulae = vec!["git".to_string(), "node".to_string(), "python".to_string()];
        let previous_formulae = vec!["git".to_string(), "vim".to_string()];
        let current_casks = vec!["docker".to_string()];
        let previous_casks = vec!["docker".to_string(), "firefox".to_string()];

        let changes = calculate_package_changes(
            &current_formulae,
            &current_casks,
            Some(&previous_formulae),
            Some(&previous_casks),
        );

        assert_eq!(changes.added_formulae, 2); // node, python
        assert_eq!(changes.removed_formulae, 1); // vim
        assert_eq!(changes.added_casks, 0);
        assert_eq!(changes.removed_casks, 1); // firefox
    }

    #[test]
    fn test_package_changes_has_changes() {
        let no_changes = PackageChanges {
            added_formulae: 0,
            removed_formulae: 0,
            added_casks: 0,
            removed_casks: 0,
        };
        assert!(!no_changes.has_changes());

        let has_changes = PackageChanges {
            added_formulae: 1,
            removed_formulae: 0,
            added_casks: 0,
            removed_casks: 0,
        };
        assert!(has_changes.has_changes());
    }

    #[test]
    fn test_package_stats_collect() -> Result<()> {
        let executor = MockBrewExecutor::new();
        let formulae = vec!["git".to_string(), "node".to_string()];
        let casks = vec!["docker".to_string()];
        let mut existing_settings = HashMap::new();
        existing_settings.insert("git".to_string(), true);
        existing_settings.insert("node".to_string(), false);
        existing_settings.insert("docker".to_string(), true);

        let previous_formulae = vec!["git".to_string()]; // node is new
        let previous_casks = vec!["docker".to_string(), "firefox".to_string()]; // firefox removed

        let stats = PackageStats::collect(
            &executor,
            &formulae,
            &casks,
            &existing_settings,
            Some(&previous_formulae),
            Some(&previous_casks),
        )?;

        assert_eq!(stats.total_formulae, 2);
        assert_eq!(stats.total_casks, 1);
        assert_eq!(stats.total_packages, 3);
        assert_eq!(stats.enabled_formulae, 1); // git
        assert_eq!(stats.disabled_formulae, 1); // node
        assert_eq!(stats.enabled_casks, 1); // docker
        assert_eq!(stats.disabled_casks, 0);

        // MockBrewExecutor has 2 outdated packages: git (formula) and docker (cask)
        assert_eq!(stats.outdated_formulae, 1);
        assert_eq!(stats.outdated_casks, 1);
        assert_eq!(stats.total_outdated, 2);

        // Changes: +1 formula (node), -1 cask (firefox)
        assert_eq!(stats.changes.added_formulae, 1);
        assert_eq!(stats.changes.removed_formulae, 0);
        assert_eq!(stats.changes.added_casks, 0);
        assert_eq!(stats.changes.removed_casks, 1);

        Ok(())
    }

    #[test]
    fn test_format_as_markdown() {
        let stats = PackageStats {
            total_formulae: 10,
            total_casks: 5,
            total_packages: 15,
            enabled_formulae: 8,
            enabled_casks: 3,
            disabled_formulae: 2,
            disabled_casks: 2,
            outdated_formulae: 2,
            outdated_casks: 1,
            total_outdated: 3,
            homebrew_version: "Homebrew 4.1.5".to_string(),
            system_info: SystemInfo {
                os_version: "macOS 14.5".to_string(),
                architecture: "Apple Silicon".to_string(),
                homebrew_prefix: "/opt/homebrew".to_string(),
            },
            changes: PackageChanges {
                added_formulae: 1,
                removed_formulae: 0,
                added_casks: 0,
                removed_casks: 1,
            },
        };

        let markdown = stats.format_as_markdown();
        assert!(markdown.contains("## Statistics"));
        assert!(markdown.contains("**Total Packages**: 15"));
        assert!(markdown.contains("**Homebrew Version**: Homebrew 4.1.5"));
        assert!(markdown.contains("**Changes Since Last Dump**"));
    }
}
