use anyhow::Result;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct OutdatedPackage {
    pub name: String,
    pub current_version: String,
    pub available_version: String,
    pub package_type: PackageType,
}

#[derive(Debug, Clone)]
pub enum PackageType {
    Formula,
    Cask,
}

pub trait BrewExecutor {
    fn verify_installation(&self) -> Result<()>;
    fn get_manually_installed_formulae(&self) -> Result<Vec<String>>;
    fn get_manually_installed_casks(&self) -> Result<Vec<String>>;
    fn get_outdated_packages(&self) -> Result<Vec<OutdatedPackage>>;
    fn upgrade_package(&self, package: &OutdatedPackage) -> Result<()>;
    fn get_version(&self) -> Result<String>;
    fn get_system_info(&self) -> Result<crate::stats::SystemInfo>;
}

pub struct SystemBrewExecutor;

impl BrewExecutor for SystemBrewExecutor {
    fn verify_installation(&self) -> Result<()> {
        let output = Command::new("brew").arg("--version").output();
        match output {
            Ok(_) => Ok(()),
            Err(_) => {
                anyhow::bail!("Homebrew is not installed or not in PATH. Please install Homebrew first: https://brew.sh/");
            }
        }
    }

    fn get_manually_installed_formulae(&self) -> Result<Vec<String>> {
        let output = Command::new("brew")
            .args(["leaves", "--installed-on-request"])
            .output()?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to get manually installed formulae: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let packages = String::from_utf8(output.stdout)?
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect();

        Ok(packages)
    }

    fn get_manually_installed_casks(&self) -> Result<Vec<String>> {
        let all_casks_output = Command::new("brew").args(["list", "--cask"]).output()?;

        if !all_casks_output.status.success() {
            anyhow::bail!(
                "Failed to get installed casks: {}",
                String::from_utf8_lossy(&all_casks_output.stderr)
            );
        }

        let all_casks: Vec<String> = String::from_utf8(all_casks_output.stdout)?
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect();

        Ok(all_casks)
    }

    fn get_outdated_packages(&self) -> Result<Vec<OutdatedPackage>> {
        let mut outdated = Vec::new();

        // Get outdated formulae
        let formulae_output = Command::new("brew")
            .args(["outdated", "--formula", "--verbose"])
            .output()?;

        if formulae_output.status.success() {
            let formulae_text = String::from_utf8(formulae_output.stdout)?;
            for line in formulae_text.lines() {
                if let Some(package) = parse_outdated_line(line, PackageType::Formula) {
                    outdated.push(package);
                }
            }
        }

        // Get outdated casks
        let casks_output = Command::new("brew")
            .args(["outdated", "--cask", "--greedy", "--verbose"])
            .output()?;

        if casks_output.status.success() {
            let casks_text = String::from_utf8(casks_output.stdout)?;
            for line in casks_text.lines() {
                if let Some(package) = parse_outdated_line(line, PackageType::Cask) {
                    outdated.push(package);
                }
            }
        }

        Ok(outdated)
    }

    fn upgrade_package(&self, package: &OutdatedPackage) -> Result<()> {
        let cmd = "upgrade";
        let args = match package.package_type {
            PackageType::Formula => vec![cmd, &package.name],
            PackageType::Cask => vec![cmd, "--cask", &package.name],
        };

        let output = Command::new("brew").args(&args).output()?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to upgrade {}: {}", package.name, error_msg);
        }

        Ok(())
    }

    fn get_version(&self) -> Result<String> {
        let output = Command::new("brew").arg("--version").output()?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to get Homebrew version: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let version_output = String::from_utf8_lossy(&output.stdout);
        let first_line = version_output.lines().next().unwrap_or("Unknown version");
        Ok(first_line.to_string())
    }

    fn get_system_info(&self) -> Result<crate::stats::SystemInfo> {
        // Get Homebrew prefix
        let homebrew_prefix = {
            let output = Command::new("brew").arg("--prefix").output()?;
            if output.status.success() {
                String::from_utf8_lossy(&output.stdout).trim().to_string()
            } else {
                "/usr/local".to_string() // fallback
            }
        };

        // Get OS version with cross-platform support
        let os_version = {
            #[cfg(target_os = "macos")]
            {
                let output = Command::new("sw_vers").arg("-productVersion").output();
                match output {
                    Ok(out) if out.status.success() => {
                        format!("macOS {}", String::from_utf8_lossy(&out.stdout).trim())
                    }
                    _ => "macOS Unknown".to_string(),
                }
            }
            #[cfg(target_os = "linux")]
            {
                // Try to read from /etc/os-release
                if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
                    if let Some(line) = content.lines().find(|l| l.starts_with("PRETTY_NAME=")) {
                        if let Some(name) = line
                            .strip_prefix("PRETTY_NAME=")
                            .map(|s| s.trim_matches('"'))
                        {
                            return Ok(crate::stats::SystemInfo {
                                os_version: name.to_string(),
                                architecture: get_architecture_safe(),
                                homebrew_prefix,
                            });
                        }
                    }
                }
                "Linux".to_string()
            }
            #[cfg(not(any(target_os = "macos", target_os = "linux")))]
            {
                "Unknown OS".to_string()
            }
        };

        // Get architecture
        let architecture = get_architecture_safe();

        Ok(crate::stats::SystemInfo {
            os_version,
            architecture,
            homebrew_prefix,
        })
    }
}

fn get_architecture_safe() -> String {
    let output = Command::new("uname").arg("-m").output();
    match output {
        Ok(out) if out.status.success() => {
            let arch = String::from_utf8_lossy(&out.stdout).trim().to_string();
            match arch.as_str() {
                "arm64" => "Apple Silicon".to_string(),
                "x86_64" => "Intel".to_string(),
                _ => arch,
            }
        }
        _ => "Unknown".to_string(),
    }
}

pub fn parse_outdated_line(line: &str, package_type: PackageType) -> Option<OutdatedPackage> {
    // Format: "package (current_version) < available_version" or "package (current_version) != available_version"
    if let Some(pos) = line.find(" (") {
        let name = line[..pos].trim().to_string();
        let rest = &line[pos + 2..];

        if let Some(end_paren) = rest.find(") ") {
            let current_version = rest[..end_paren].to_string();
            let remainder = &rest[end_paren + 2..].trim();

            // Skip the comparison operator (< or !=) and get the available version
            if let Some(space_pos) = remainder.find(' ') {
                let available_version = remainder[space_pos + 1..].trim().to_string();

                return Some(OutdatedPackage {
                    name,
                    current_version,
                    available_version,
                    package_type,
                });
            }
        }
    }

    None
}

pub struct MockBrewExecutor {
    formulae: Vec<String>,
    casks: Vec<String>,
    outdated_packages: Vec<OutdatedPackage>,
    should_fail_verification: bool,
}

impl Default for MockBrewExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl MockBrewExecutor {
    pub fn new() -> Self {
        Self {
            formulae: vec!["git".to_string(), "node".to_string(), "python".to_string()],
            casks: vec![
                "visual-studio-code".to_string(),
                "docker".to_string(),
                "firefox".to_string(),
            ],
            outdated_packages: vec![
                OutdatedPackage {
                    name: "git".to_string(),
                    current_version: "2.40.0".to_string(),
                    available_version: "2.41.0".to_string(),
                    package_type: PackageType::Formula,
                },
                OutdatedPackage {
                    name: "docker".to_string(),
                    current_version: "4.18.0".to_string(),
                    available_version: "4.19.0".to_string(),
                    package_type: PackageType::Cask,
                },
            ],
            should_fail_verification: false,
        }
    }

    pub fn with_failed_verification(mut self) -> Self {
        self.should_fail_verification = true;
        self
    }

    pub fn with_formulae(mut self, formulae: Vec<String>) -> Self {
        self.formulae = formulae;
        self
    }

    pub fn with_casks(mut self, casks: Vec<String>) -> Self {
        self.casks = casks;
        self
    }

    #[allow(dead_code)]
    pub fn with_outdated_packages(mut self, packages: Vec<OutdatedPackage>) -> Self {
        self.outdated_packages = packages;
        self
    }
}

impl BrewExecutor for MockBrewExecutor {
    fn verify_installation(&self) -> Result<()> {
        if self.should_fail_verification {
            anyhow::bail!("Homebrew is not installed or not in PATH. Please install Homebrew first: https://brew.sh/");
        }
        Ok(())
    }

    fn get_manually_installed_formulae(&self) -> Result<Vec<String>> {
        Ok(self.formulae.clone())
    }

    fn get_manually_installed_casks(&self) -> Result<Vec<String>> {
        Ok(self.casks.clone())
    }

    fn get_outdated_packages(&self) -> Result<Vec<OutdatedPackage>> {
        Ok(self.outdated_packages.clone())
    }

    fn upgrade_package(&self, _package: &OutdatedPackage) -> Result<()> {
        Ok(())
    }

    fn get_version(&self) -> Result<String> {
        Ok("Homebrew 4.1.5".to_string())
    }

    fn get_system_info(&self) -> Result<crate::stats::SystemInfo> {
        Ok(crate::stats::SystemInfo {
            os_version: "macOS 14.5".to_string(),
            architecture: "Apple Silicon".to_string(),
            homebrew_prefix: "/usr/local".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_outdated_line_formula() {
        let line = "git (2.40.0) < 2.41.0";
        let result = parse_outdated_line(line, PackageType::Formula);

        assert!(result.is_some());
        let package = result.unwrap();
        assert_eq!(package.name, "git");
        assert_eq!(package.current_version, "2.40.0");
        assert_eq!(package.available_version, "2.41.0");
        assert!(matches!(package.package_type, PackageType::Formula));
    }

    #[test]
    fn test_parse_outdated_line_cask() {
        let line = "visual-studio-code (1.79.0) != 1.80.0";
        let result = parse_outdated_line(line, PackageType::Cask);

        assert!(result.is_some());
        let package = result.unwrap();
        assert_eq!(package.name, "visual-studio-code");
        assert_eq!(package.current_version, "1.79.0");
        assert_eq!(package.available_version, "1.80.0");
        assert!(matches!(package.package_type, PackageType::Cask));
    }

    #[test]
    fn test_parse_outdated_line_invalid() {
        let line = "invalid line format";
        let result = parse_outdated_line(line, PackageType::Formula);
        assert!(result.is_none());
    }

    #[test]
    fn test_mock_brew_executor() -> Result<()> {
        let executor = MockBrewExecutor::new();

        // Test verification
        assert!(executor.verify_installation().is_ok());

        // Test formulae
        let formulae = executor.get_manually_installed_formulae()?;
        assert_eq!(formulae.len(), 3);
        assert!(formulae.contains(&"git".to_string()));

        // Test casks
        let casks = executor.get_manually_installed_casks()?;
        assert_eq!(casks.len(), 3);
        assert!(casks.contains(&"docker".to_string()));

        // Test outdated packages
        let outdated = executor.get_outdated_packages()?;
        assert_eq!(outdated.len(), 2);

        Ok(())
    }

    #[test]
    fn test_mock_brew_executor_with_failed_verification() {
        let executor = MockBrewExecutor::new().with_failed_verification();
        assert!(executor.verify_installation().is_err());
    }

    #[test]
    fn test_mock_brew_executor_with_custom_data() -> Result<()> {
        let custom_formulae = vec!["custom-formula".to_string()];
        let custom_casks = vec!["custom-cask".to_string()];

        let executor = MockBrewExecutor::new()
            .with_formulae(custom_formulae.clone())
            .with_casks(custom_casks.clone());

        let formulae = executor.get_manually_installed_formulae()?;
        let casks = executor.get_manually_installed_casks()?;

        assert_eq!(formulae, custom_formulae);
        assert_eq!(casks, custom_casks);

        Ok(())
    }
}
