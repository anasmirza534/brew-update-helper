use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "brew-update-helper")]
#[command(about = "A CLI tool for selective Homebrew package upgrade management")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Show what would be done without executing
    #[arg(long)]
    pub dry_run: bool,

    /// Specify custom config file path
    #[arg(long)]
    pub config: Option<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Generate/update package selection settings
    Dump,
    /// Upgrade selected packages interactively
    Upgrade,
}
