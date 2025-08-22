use anyhow::Result;
use chrono::Utc;
use clap::{Parser, Subcommand};
use std::fs::{self, OpenOptions};
use std::io::Write;
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
    let outdated_packages = get_outdated_packages()?;

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
    execute_upgrades(&selected_packages, cli.dry_run)?;

    Ok(())
}

#[derive(Debug, Clone)]
struct OutdatedPackage {
    name: String,
    current_version: String,
    available_version: String,
    package_type: PackageType,
}

#[derive(Debug, Clone)]
enum PackageType {
    Formula,
    Cask,
}

fn get_outdated_packages() -> Result<Vec<OutdatedPackage>> {
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

fn parse_outdated_line(line: &str, package_type: PackageType) -> Option<OutdatedPackage> {
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

fn show_interactive_selection(packages: &[&OutdatedPackage]) -> Result<Vec<OutdatedPackage>> {
    use crossterm::{
        event::{self, Event, KeyCode, KeyEventKind},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use ratatui::{
        backend::CrosstermBackend,
        layout::{Constraint, Direction, Layout},
        style::{Color, Modifier, Style},
        text::{Line, Span},
        widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
        Terminal,
    };

    // Track selection state
    let mut selected: Vec<bool> = vec![true; packages.len()];
    let mut list_state = ListState::default();
    list_state.select(Some(0));

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(2),
                ])
                .split(f.size());

            // Header
            let header = Paragraph::new("Outdated packages found - Select packages to upgrade")
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(header, chunks[0]);

            // Package list
            let items: Vec<ListItem> = packages
                .iter()
                .enumerate()
                .map(|(i, pkg)| {
                    let checkbox = if selected[i] { "[x]" } else { "[ ]" };
                    let type_str = match pkg.package_type {
                        PackageType::Formula => "Formula",
                        PackageType::Cask => "Cask",
                    };

                    let type_text = format!("({}) ", type_str);
                    let version_text =
                        format!("{} → {}", pkg.current_version, pkg.available_version);

                    let content = Line::from(vec![
                        Span::styled(checkbox, Style::default().fg(Color::Green)),
                        Span::raw(" "),
                        Span::styled(&pkg.name, Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(" "),
                        Span::styled(type_text, Style::default().fg(Color::Blue)),
                        Span::raw(version_text),
                    ]);

                    ListItem::new(content)
                })
                .collect();

            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL))
                .highlight_style(Style::default().bg(Color::DarkGray));

            f.render_stateful_widget(list, chunks[1], &mut list_state);

            // Footer
            let footer = Paragraph::new("↑↓: Navigate, SPACE: Toggle, ENTER: Proceed, q: Quit")
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(footer, chunks[2]);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') => {
                        disable_raw_mode()?;
                        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                        return Ok(vec![]);
                    }
                    KeyCode::Up => {
                        let i = list_state.selected().unwrap_or(0);
                        if i > 0 {
                            list_state.select(Some(i - 1));
                        }
                    }
                    KeyCode::Down => {
                        let i = list_state.selected().unwrap_or(0);
                        if i < packages.len() - 1 {
                            list_state.select(Some(i + 1));
                        }
                    }
                    KeyCode::Char(' ') => {
                        if let Some(i) = list_state.selected() {
                            selected[i] = !selected[i];
                        }
                    }
                    KeyCode::Enter => {
                        disable_raw_mode()?;
                        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                        let result = packages
                            .iter()
                            .enumerate()
                            .filter(|(i, _)| selected[*i])
                            .map(|(_, pkg)| (*pkg).clone())
                            .collect();
                        return Ok(result);
                    }
                    _ => {}
                }
            }
        }
    }
}

fn execute_upgrades(packages: &[OutdatedPackage], dry_run: bool) -> Result<()> {
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
        let cmd = match pkg.package_type {
            PackageType::Formula => "upgrade",
            PackageType::Cask => "upgrade",
        };

        let args = match pkg.package_type {
            PackageType::Formula => vec![cmd, &pkg.name],
            PackageType::Cask => vec![cmd, "--cask", &pkg.name],
        };

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
            let output = Command::new("brew").args(&args).output()?;

            if !output.status.success() {
                let error_msg = String::from_utf8_lossy(&output.stderr);
                eprintln!("    ❌ Failed to upgrade {}: {}", pkg.name, error_msg);
                log_operation(&format!(
                    "FAILED: {} {} → {} - {}",
                    pkg.name, pkg.current_version, pkg.available_version, error_msg
                ))?;
                failed_upgrades += 1;
            } else {
                println!("    ✅ Successfully upgraded {}", pkg.name);
                log_operation(&format!(
                    "SUCCESS: {} {} → {}",
                    pkg.name, pkg.current_version, pkg.available_version
                ))?;
                successful_upgrades += 1;
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

fn show_simple_selection(packages: &[&OutdatedPackage]) -> Result<Vec<OutdatedPackage>> {
    println!("\nOutdated packages found:");

    for (i, pkg) in packages.iter().enumerate() {
        let type_str = match pkg.package_type {
            PackageType::Formula => "Formula",
            PackageType::Cask => "Cask",
        };
        println!(
            "{}. [x] {} ({}) {} → {}",
            i + 1,
            pkg.name,
            type_str,
            pkg.current_version,
            pkg.available_version
        );
    }

    println!("\nAll packages are selected by default.");
    println!(
        "Do you want to proceed with upgrading all {} packages? (y/n): ",
        packages.len()
    );

    use std::io::{self, Write};
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    if input.trim().to_lowercase().starts_with('y') {
        Ok(packages.iter().map(|pkg| (*pkg).clone()).collect())
    } else {
        Ok(vec![])
    }
}

fn log_operation(message: &str) -> Result<()> {
    let log_path = get_log_path()?;

    // Ensure log directory exists
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
    let log_entry = format!("[{}] {}\n", timestamp, message);

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;

    file.write_all(log_entry.as_bytes())?;
    file.flush()?;

    Ok(())
}

fn get_log_path() -> Result<PathBuf> {
    // For testing, use current directory
    if std::env::var("CARGO_MANIFEST_DIR").is_ok() {
        return Ok(PathBuf::from("./brew-update-helper.log"));
    }

    // Production: use ~/.config/brew-update-helper/upgrade.log
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?
        .join("brew-update-helper");

    Ok(config_dir.join("upgrade.log"))
}
