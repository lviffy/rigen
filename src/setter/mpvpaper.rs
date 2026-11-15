use anyhow::{Context, Result};
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

use crate::config::Config;

/// Set wallpaper using mpvpaper.
///
/// mpvpaper forks per output. We kill any existing mpvpaper instances first,
/// wait briefly for clean shutdown, then launch mpvpaper for each active output.
pub async fn set(config: &Config, image_path: &str, monitors: &[String]) -> Result<()> {
    // Kill existing mpvpaper instances
    let _ = Command::new("pkill")
        .args(["-x", "mpvpaper"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await;

    // Small delay to allow previous processes to exit
    tokio::time::sleep(Duration::from_millis(100)).await;

    let default_args = vec![
        "-f".to_string(),
        "--auto-pause".to_string(),
        "-o".to_string(),
        "no-audio loop".to_string(),
    ];

    let extra_args = if config.wallpaper.mpvpaper.args.is_empty() {
        &default_args
    } else {
        &config.wallpaper.mpvpaper.args
    };

    // If monitors list has "*", just run once for "*"
    let targets: Vec<&str> = if monitors.is_empty() || monitors.iter().any(|m| m == "*") {
        vec!["*"]
    } else {
        monitors.iter().map(|s| s.as_str()).collect()
    };

    for output in targets {
        let mut cmd = Command::new("mpvpaper");
        for arg in extra_args {
            cmd.arg(arg);
        }
        cmd.arg(output);
        cmd.arg(image_path);
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());

        cmd.status()
            .await
            .with_context(|| format!("Failed to spawn mpvpaper for output {}", output))?;
    }

    Ok(())
}

