use clap::{Parser, Subcommand};
use anyhow::Result;

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

    let output = Command::new("brew")
        .arg("--version")
        .output();

    match output {
        Ok(_) => Ok(()),
        Err(_) => {
            anyhow::bail!("Homebrew is not installed or not in PATH. Please install Homebrew first: https://brew.sh/");
        }
    }
}

fn dump_command(cli: &Cli) -> Result<()> {
    println!("Dump command not yet implemented");
    println!("Config file: {:?}", cli.config);
    Ok(())
}

fn upgrade_command(cli: &Cli) -> Result<()> {
    println!("Upgrade command not yet implemented");
    println!("Config file: {:?}", cli.config);
    Ok(())
}
