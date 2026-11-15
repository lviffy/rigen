pub mod hyprpaper;
pub mod mpvpaper;
pub mod quickshell;
pub mod swww;

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;

use crate::config::Config;

/// Known wallpaper backends.
#[derive(Debug, Clone, PartialEq)]
pub enum Backend {
    Quickshell,
    Swww,
    Hyprpaper,
    Mpvpaper,
    Feh,
    None,
}

/// A connected monitor.
#[derive(Debug, Clone, Deserialize)]
pub struct Monitor {
    pub name: String,
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
}

/// Query active monitor names via `hyprctl monitors -j`.
pub async fn query_monitors() -> Result<Vec<String>> {
    let output = Command::new("hyprctl")
        .args(["monitors", "-j"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await;

    match output {
        Ok(out) if out.status.success() => {
            let monitors: Vec<Monitor> = serde_json::from_slice(&out.stdout)
                .unwrap_or_default();
            let names: Vec<String> = monitors.into_iter().map(|m| m.name).collect();
            if names.is_empty() {
                Ok(vec!["eDP-1".into()])
            } else {
                Ok(names)
            }
        }
        _ => Ok(vec!["*".into()]),
    }
}

/// Query maximum monitor resolution via `hyprctl monitors -j`.
pub async fn query_max_resolution() -> (u32, u32) {
    let output = Command::new("hyprctl")
        .args(["monitors", "-j"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await;

    if let Ok(out) = output {
        if out.status.success() {
            if let Ok(monitors) = serde_json::from_slice::<Vec<Monitor>>(&out.stdout) {
                let max_w = monitors.iter().map(|m| m.width).max().unwrap_or(1920);
                let max_h = monitors.iter().map(|m| m.height).max().unwrap_or(1080);
                return (max_w.max(1920), max_h.max(1080));
            }
        }
    }
    (1920, 1080)
}

/// Upscale / scale image to match monitor resolution if smaller, ensuring
/// switchwall.sh won't prompt for upscale and wallpaper is crisp.
pub fn postprocess_wallpaper(
    source_path: &Path,
    dest_path: &Path,
    target_width: u32,
    target_height: u32,
) -> Result<()> {
    let img = image::open(source_path)
        .with_context(|| format!("Failed to open downloaded image at {}", source_path.display()))?;

    let (w, h) = (img.width(), img.height());
    let final_img = if w < target_width || h < target_height {
        img.resize_to_fill(target_width, target_height, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };

    if let Some(parent) = dest_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    final_img
        .save(dest_path)
        .with_context(|| format!("Failed to save final wallpaper to {}", dest_path.display()))?;

    Ok(())
}

/// Check if QuickShell or AGS desktop shell is running.
pub async fn is_quickshell_active() -> bool {
    // 1. Quickshell process (end-4 dots v3/v4)
    if let Ok(s) = Command::new("pgrep").args(["-x", "qs"]).stdout(Stdio::null()).stderr(Stdio::null()).status().await {
        if s.success() {
            return true;
        }
    }

    // 2. AGS process (legacy end-4 dots v1/v2)
    if let Ok(s) = Command::new("pgrep").args(["-x", "ags"]).stdout(Stdio::null()).stderr(Stdio::null()).status().await {
        if s.success() {
            return true;
        }
    }

    false
}

/// Auto-detect the best available backend.
pub async fn detect_backend() -> Backend {
    // 1. Quickshell: if running, it renders the desktop layer on top of all others!
    if is_quickshell_active().await {
        return Backend::Quickshell;
    }

    // 2. swww (check if daemon is running)
    if let Ok(out) = Command::new("swww")
        .arg("query")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
    {
        if out.success() {
            return Backend::Swww;
        }
    }

    // 3. hyprpaper
    if which::which("hyprpaper").is_ok() {
        return Backend::Hyprpaper;
    }

    // 4. mpvpaper
    if which::which("mpvpaper").is_ok() {
        return Backend::Mpvpaper;
    }

    // 5. feh (X11 fallback)
    if which::which("feh").is_ok() {
        return Backend::Feh;
    }

    Backend::None
}

/// Resolve the backend from config string.
pub async fn resolve_backend(config: &Config) -> Backend {
    match config.wallpaper.backend.as_str() {
        "quickshell" => Backend::Quickshell,
        "mpvpaper" => Backend::Mpvpaper,
        "swww" => Backend::Swww,
        "hyprpaper" => Backend::Hyprpaper,
        "feh" => Backend::Feh,
        "none" => Backend::None,
        "auto" | _ => detect_backend().await,
    }
}

/// Set the wallpaper using the appropriate backend with automatic fallbacks.
pub async fn set_wallpaper(
    config: &Config,
    backend: &Backend,
    image_path: &Path,
) -> Result<()> {
    let path_str = image_path
        .to_str()
        .context("Image path is not valid UTF-8")?;

    let monitors = query_monitors().await?;

    match backend {
        Backend::Quickshell => {
            if let Err(e) = quickshell::set(path_str).await {
                // If quickshell IPC / script is not available, fall back to swww or hyprpaper
                if which::which("swww").is_ok() {
                    swww::set(config, path_str).await?;
                } else if which::which("hyprpaper").is_ok() {
                    hyprpaper::set(path_str, &monitors).await?;
                } else if which::which("mpvpaper").is_ok() {
                    mpvpaper::set(config, path_str, &monitors).await?;
                } else {
                    return Err(e);
                }
            }
        }
        Backend::Mpvpaper => {
            mpvpaper::set(config, path_str, &monitors).await?;
        }
        Backend::Swww => {
            swww::set(config, path_str).await?;
        }
        Backend::Hyprpaper => {
            hyprpaper::set(path_str, &monitors).await?;
        }
        Backend::Feh => {
            Command::new("feh")
                .args(["--bg-fill", path_str])
                .status()
                .await
                .context("Failed to run feh")?;
        }
        Backend::None => {}
    }

    Ok(())
}

/// Run post-hooks: matugen, custom hook, notification.
pub async fn run_post_hooks(
    config: &Config,
    image_path: &Path,
    prompt: &str,
    sync_matugen: bool,
    notify: bool,
) -> Result<()> {
    let path_str = image_path
        .to_str()
        .context("Image path is not valid UTF-8")?;

    // matugen sync
    if sync_matugen || config.wallpaper.sync_matugen {
        if which::which("matugen").is_ok() {
            let status = Command::new("matugen")
                .args(["image", path_str])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await;
            if let Err(e) = status {
                eprintln!("Warning: matugen failed: {}", e);
            }
        }
    }

    // Custom post_hook
    if let Some(hook) = &config.wallpaper.post_hook {
        let _ = Command::new("sh")
            .args(["-c", hook])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;
    }

    // Desktop notification
    if notify {
        crate::notify::send(path_str, prompt).await;
    }

    Ok(())
}
