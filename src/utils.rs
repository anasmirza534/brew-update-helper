use anyhow::Result;
use chrono::Utc;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

pub fn log_operation(message: &str) -> Result<()> {
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

pub fn get_log_path() -> Result<PathBuf> {
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
