use anyhow::{Context, Result};
use std::process::Stdio;
use tokio::process::Command;

use crate::config::Config;

/// Set wallpaper using swww.
///
/// Executes `swww img <path> <swww_args>`.
pub async fn set(config: &Config, image_path: &str) -> Result<()> {
    let mut cmd = Command::new("swww");
    cmd.arg("img");
    cmd.arg(image_path);

    let default_args = vec![
        "--transition-type".to_string(),
        "wipe".to_string(),
        "--transition-angle".to_string(),
        "30".to_string(),
        "--transition-fps".to_string(),
        "120".to_string(),
    ];

    let args = if config.wallpaper.swww.args.is_empty() {
        &default_args
    } else {
        &config.wallpaper.swww.args
    };

    for arg in args {
        cmd.arg(arg);
    }

    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());

    let status = cmd
        .status()
        .await
        .context("Failed to execute swww command")?;

    if !status.success() {
        anyhow::bail!("swww img exited with non-zero status");
    }

    Ok(())
}

