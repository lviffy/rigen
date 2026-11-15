use anyhow::{Context, Result};
use std::process::Stdio;
use tokio::process::Command;

/// Set wallpaper using hyprpaper via hyprctl IPC.
///
/// 1. `hyprctl hyprpaper preload <path>`
/// 2. `hyprctl hyprpaper wallpaper "<monitor>,<path>"` for each monitor (or empty monitor string for all)
/// 3. `hyprctl hyprpaper unload all`
pub async fn set(image_path: &str, monitors: &[String]) -> Result<()> {
    // 1. Preload image
    let preload_status = Command::new("hyprctl")
        .args(["hyprpaper", "preload", image_path])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .context("Failed to execute `hyprctl hyprpaper preload`")?;

    if !preload_status.success() {
        anyhow::bail!("hyprctl hyprpaper preload failed");
    }

    // 2. Set on monitors
    let targets: Vec<&str> = if monitors.is_empty() || monitors.iter().any(|m| m == "*") {
        vec![""]
    } else {
        monitors.iter().map(|s| s.as_str()).collect()
    };

    for target in targets {
        let wall_arg = if target.is_empty() {
            format!(",{}", image_path)
        } else {
            format!("{},{}", target, image_path)
        };

        Command::new("hyprctl")
            .args(["hyprpaper", "wallpaper", &wall_arg])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .with_context(|| format!("Failed to set hyprpaper wallpaper for target '{}'", target))?;
    }

    // 3. Unload old images to prevent VRAM accumulation
    let _ = Command::new("hyprctl")
        .args(["hyprpaper", "unload", "all"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await;

    Ok(())
}

