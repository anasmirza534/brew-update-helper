pub mod brew;
pub mod cli;
pub mod commands;
pub mod config;
pub mod ui;
pub mod utils;

// Re-export main types for convenience
pub use brew::{BrewExecutor, OutdatedPackage, PackageType};
pub use cli::{Cli, Commands};
pub use config::{generate_settings_content, get_config_path, read_existing_settings};
pub use utils::{get_log_path, log_operation};

use anyhow::Result;
use clap::Parser;

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    let executor = create_executor();

    executor.verify_installation()?;

    match cli.command {
        Commands::Dump => {
            println!("Running dump command...");
            if cli.dry_run {
                println!("(dry run mode)");
            }
            commands::dump_command(&cli, &*executor)?;
        }
        Commands::Upgrade => {
            println!("Running upgrade command...");
            if cli.dry_run {
                println!("(dry run mode)");
            }
            commands::upgrade_command(&cli, &*executor)?;
        }
    }

    Ok(())
}

fn create_executor() -> Box<dyn BrewExecutor> {
    // Use mock executor in CI environments or when explicitly requested
    if std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok() || std::env::var("MOCK_BREW").is_ok() {
        return Box::new(brew::MockBrewExecutor::new());
    }

    Box::new(brew::SystemBrewExecutor)
}
