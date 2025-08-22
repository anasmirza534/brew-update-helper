use anyhow::Result;
use chrono::Utc;
use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser)]
#[command(name = "brew-update-helper")]
#[command(about = "A CLI tool for selective Homebrew package upgrade management")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Show what would be done without executing
    #[arg(long)]
    dry_run: bool,

    /// Specify custom config file path
    #[arg(long)]
    config: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate/update package selection settings
    Dump,
    /// Upgrade selected packages interactively
    Upgrade,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Verify brew is installed
    verify_brew_installation()?;

    match cli.command {
        Commands::Dump => {
            println!("Running dump command...");
            if cli.dry_run {
                println!("(dry run mode)");
            }
            dump_command(&cli)?;
        }
        Commands::Upgrade => {
            println!("Running upgrade command...");
            if cli.dry_run {
                println!("(dry run mode)");
            }
            upgrade_command(&cli)?;
        }
    }

    Ok(())
}

fn verify_brew_installation() -> Result<()> {
    use std::process::Command;

    let output = Command::new("brew").arg("--version").output();

    match output {
        Ok(_) => Ok(()),
        Err(_) => {
            anyhow::bail!("Homebrew is not installed or not in PATH. Please install Homebrew first: https://brew.sh/");
        }
    }
}

fn dump_command(cli: &Cli) -> Result<()> {
    let config_path = get_config_path(&cli.config)?;

    if cli.dry_run {
        println!("Would write settings to: {}", config_path.display());
    }

    // Get manually installed formulae
    let formulae = get_manually_installed_formulae()?;
    println!("Found {} manually installed formulae", formulae.len());

    // Get manually installed casks
    let casks = get_manually_installed_casks()?;
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

fn get_config_path(custom_path: &Option<String>) -> Result<PathBuf> {
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

fn get_manually_installed_formulae() -> Result<Vec<String>> {
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

fn get_manually_installed_casks() -> Result<Vec<String>> {
    // First get all installed casks
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

fn read_existing_settings(
    config_path: &PathBuf,
) -> Result<std::collections::HashMap<String, bool>> {
    let mut settings = std::collections::HashMap::new();

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

fn generate_settings_content(
    formulae: &[String],
    casks: &[String],
    existing_settings: &std::collections::HashMap<String, bool>,
) -> String {
    let mut content = String::new();

    content.push_str("# Brew Auto-Update Settings\n\n");
    content.push_str(&format!(
        "Generated on: {}\n\n",
        Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    ));

    // Formulae section
    content.push_str("## Formulae\n\n");
    for formula in formulae {
        let enabled = existing_settings.get(formula).copied().unwrap_or(true);
        let checkbox = if enabled { "[x]" } else { "[ ]" };
        content.push_str(&format!("- {} {}\n", checkbox, formula));
    }

    // Casks section
    content.push_str("\n## Casks\n\n");
    for cask in casks {
        let enabled = existing_settings.get(cask).copied().unwrap_or(true);
        let checkbox = if enabled { "[x]" } else { "[ ]" };
        content.push_str(&format!("- {} {}\n", checkbox, cask));
    }

    content
}

fn upgrade_command(cli: &Cli) -> Result<()> {
    println!("Upgrade command not yet implemented");
    println!("Config file: {:?}", cli.config);
    Ok(())
}
