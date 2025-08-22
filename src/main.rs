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

struct TerminalGuard;

impl TerminalGuard {
    fn new() -> Result<Self> {
        use crossterm::terminal::enable_raw_mode;
        enable_raw_mode()?;
        Ok(TerminalGuard)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        use crossterm::{
            execute,
            terminal::{disable_raw_mode, LeaveAlternateScreen},
        };
        use std::io::{self, Write};

        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        let _ = io::stdout().flush();
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let executor = create_executor();

    // Verify brew is installed
    executor.verify_installation()?;

    match cli.command {
        Commands::Dump => {
            println!("Running dump command...");
            if cli.dry_run {
                println!("(dry run mode)");
            }
            dump_command(&cli, &*executor)?;
        }
        Commands::Upgrade => {
            println!("Running upgrade command...");
            if cli.dry_run {
                println!("(dry run mode)");
            }
            upgrade_command(&cli, &*executor)?;
        }
    }

    Ok(())
}

fn create_executor() -> Box<dyn BrewExecutor> {
    #[cfg(test)]
    {
        if std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok() {
            return Box::new(MockBrewExecutor::new());
        }
    }

    Box::new(SystemBrewExecutor)
}

fn dump_command(cli: &Cli, executor: &dyn BrewExecutor) -> Result<()> {
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

fn upgrade_command(cli: &Cli, executor: &dyn BrewExecutor) -> Result<()> {
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

trait BrewExecutor {
    fn verify_installation(&self) -> Result<()>;
    fn get_manually_installed_formulae(&self) -> Result<Vec<String>>;
    fn get_manually_installed_casks(&self) -> Result<Vec<String>>;
    fn get_outdated_packages(&self) -> Result<Vec<OutdatedPackage>>;
    fn upgrade_package(&self, package: &OutdatedPackage) -> Result<()>;
}

struct SystemBrewExecutor;

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
}

#[cfg(test)]
struct MockBrewExecutor {
    formulae: Vec<String>,
    casks: Vec<String>,
    outdated_packages: Vec<OutdatedPackage>,
    should_fail_verification: bool,
}

#[cfg(test)]
impl MockBrewExecutor {
    fn new() -> Self {
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

    fn with_failed_verification(mut self) -> Self {
        self.should_fail_verification = true;
        self
    }

    fn with_formulae(mut self, formulae: Vec<String>) -> Self {
        self.formulae = formulae;
        self
    }

    fn with_casks(mut self, casks: Vec<String>) -> Self {
        self.casks = casks;
        self
    }

    #[allow(dead_code)]
    fn with_outdated_packages(mut self, packages: Vec<OutdatedPackage>) -> Self {
        self.outdated_packages = packages;
        self
    }
}

#[cfg(test)]
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
    // Skip TUI in test environments to avoid terminal state issues
    if std::env::var("CI").is_ok()
        || std::env::var("GITHUB_ACTIONS").is_ok()
        || std::env::var("CARGO_TEST").is_ok()
        || cfg!(test)
    {
        return show_simple_selection(packages);
    }

    use crossterm::{
        event::{self, Event, KeyCode, KeyEventKind},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen},
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

    // Setup terminal with proper cleanup handling
    let _guard = TerminalGuard::new()?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

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
    fn test_generate_settings_content() {
        let formulae = vec!["git".to_string(), "node".to_string()];
        let casks = vec!["docker".to_string(), "firefox".to_string()];
        let mut existing_settings = HashMap::new();
        existing_settings.insert("git".to_string(), true);
        existing_settings.insert("node".to_string(), false);
        existing_settings.insert("docker".to_string(), false);

        let content = generate_settings_content(&formulae, &casks, &existing_settings);

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
    fn test_get_config_path_custom() -> Result<()> {
        let custom_path = Some("/custom/path/settings.md".to_string());
        let path = get_config_path(&custom_path)?;
        assert_eq!(path, PathBuf::from("/custom/path/settings.md"));
        Ok(())
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
