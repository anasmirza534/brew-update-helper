use anyhow::Result;
use std::fs;

use crate::brew::{BrewExecutor, OutdatedPackage};
use crate::cli::Cli;
use crate::config::{generate_settings_content, get_config_path, read_existing_settings};
use crate::ui::{show_interactive_selection, show_simple_selection};
use crate::utils::log_operation;

pub fn dump_command(cli: &Cli, executor: &dyn BrewExecutor) -> Result<()> {
    let config_path = get_config_path(&cli.config)?;

    if cli.dry_run {
        println!("Would write settings to: {}", config_path.display());
    }

    // Get manually installed formulae
    let formulae = executor.get_manually_installed_formulae()?;
    println!("Found {} manually installed formulae", formulae.len());

    // Get manually installed casks
    let casks = executor.get_manually_installed_casks()?;
    println!("Found {} manually installed casks", casks.len());

    // Read existing settings to preserve user selections
    let existing_settings = read_existing_settings(&config_path)?;

    // Generate new settings content
    let settings_content = generate_settings_content(&formulae, &casks, &existing_settings);

    if cli.dry_run {
        println!("\nSettings content would be:");
        println!("{}", settings_content);
    } else {
        // Ensure config directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write settings file
        fs::write(&config_path, settings_content)?;
        println!("Settings written to: {}", config_path.display());
    }

    Ok(())
}

pub fn upgrade_command(cli: &Cli, executor: &dyn BrewExecutor) -> Result<()> {
    let config_path = get_config_path(&cli.config)?;

    // Read settings file
    if !config_path.exists() {
        anyhow::bail!(
            "Settings file not found at {}. Run 'dump' command first.",
            config_path.display()
        );
    }

    let settings = read_existing_settings(&config_path)?;
    if settings.is_empty() {
        anyhow::bail!("No packages found in settings file. Run 'dump' command first.");
    }

    // Get enabled packages
    let enabled_packages: Vec<String> = settings
        .iter()
        .filter(|(_, &enabled)| enabled)
        .map(|(package, _)| package.clone())
        .collect();

    if enabled_packages.is_empty() {
        println!("No packages are enabled for upgrade in settings.");
        return Ok(());
    }

    println!("Checking for outdated packages...");

    // Get outdated packages
    let outdated_packages = executor.get_outdated_packages()?;

    // Filter to only enabled and outdated packages
    let upgradeable_packages: Vec<&OutdatedPackage> = outdated_packages
        .iter()
        .filter(|pkg| enabled_packages.contains(&pkg.name))
        .collect();

    if upgradeable_packages.is_empty() {
        println!("All enabled packages are up to date!");
        return Ok(());
    }

    // Show interactive selection (fallback to simple prompt if TUI fails)
    let selected_packages = match show_interactive_selection(&upgradeable_packages) {
        Ok(packages) => packages,
        Err(_) => {
            // Fallback to simple text-based selection
            show_simple_selection(&upgradeable_packages)?
        }
    };

    if selected_packages.is_empty() {
        println!("No packages selected for upgrade.");
        return Ok(());
    }

    // Execute upgrades
    execute_upgrades(&selected_packages, cli.dry_run, executor)?;

    Ok(())
}

fn execute_upgrades(
    packages: &[OutdatedPackage],
    dry_run: bool,
    executor: &dyn BrewExecutor,
) -> Result<()> {
    println!(
        "\n{} upgrade for {} packages:",
        if dry_run {
            "Would execute"
        } else {
            "Executing"
        },
        packages.len()
    );

    if !dry_run {
        log_operation(&format!("Starting upgrade of {} packages", packages.len()))?;
    }

    let mut successful_upgrades = 0;
    let mut failed_upgrades = 0;

    for pkg in packages {
        println!(
            "  {} {} {} → {}",
            if dry_run {
                "Would upgrade"
            } else {
                "Upgrading"
            },
            pkg.name,
            pkg.current_version,
            pkg.available_version
        );

        if !dry_run {
            match executor.upgrade_package(pkg) {
                Ok(_) => {
                    println!("    ✅ Successfully upgraded {}", pkg.name);
                    log_operation(&format!(
                        "SUCCESS: {} {} → {}",
                        pkg.name, pkg.current_version, pkg.available_version
                    ))?;
                    successful_upgrades += 1;
                }
                Err(e) => {
                    eprintln!("    ❌ Failed to upgrade {}: {}", pkg.name, e);
                    log_operation(&format!(
                        "FAILED: {} {} → {} - {}",
                        pkg.name, pkg.current_version, pkg.available_version, e
                    ))?;
                    failed_upgrades += 1;
                }
            }
        }
    }

    if dry_run {
        println!("\nDry run completed. Use without --dry-run to execute upgrades.");
    } else {
        println!(
            "\nUpgrade completed! {} successful, {} failed",
            successful_upgrades, failed_upgrades
        );
        log_operation(&format!(
            "Upgrade session completed: {} successful, {} failed",
            successful_upgrades, failed_upgrades
        ))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brew::MockBrewExecutor;
    use crate::cli::Commands;
    use tempfile::TempDir;

    #[test]
    fn test_dump_command_with_mock() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join("settings.md");

        let executor = MockBrewExecutor::new();
        let cli = Cli {
            command: Commands::Dump,
            dry_run: false,
            config: Some(config_path.to_string_lossy().to_string()),
        };

        dump_command(&cli, &executor)?;

        assert!(config_path.exists());
        let content = std::fs::read_to_string(&config_path)?;
        assert!(content.contains("# Brew Auto-Update Settings"));
        assert!(content.contains("git"));
        assert!(content.contains("docker"));

        Ok(())
    }
}
