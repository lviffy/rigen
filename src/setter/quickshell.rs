use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Stdio;

use anyhow::Result;
use tokio::process::Command;

/// Gather all possible Quickshell configuration names for end-4 dots variants.
fn discover_qs_config_names() -> Vec<String> {
    let mut names = Vec::new();
    let mut seen = HashSet::new();

    // 1. Check Hyprland / environment variables set by end-4 dots
    if let Ok(c) = std::env::var("qsConfig") {
        if seen.insert(c.clone()) {
            names.push(c);
        }
    }
    if let Ok(c) = std::env::var("QS_CONFIG_NAME") {
        if seen.insert(c.clone()) {
            names.push(c);
        }
    }

    // 2. Common defaults in end-4 dots
    for default_name in ["lviffy-shell", "ii", "illogical-impulse"] {
        if seen.insert(default_name.to_string()) {
            names.push(default_name.to_string());
        }
    }

    // 3. Scan ~/.config/quickshell for any folders containing shell.qml
    if let Some(home) = dirs::home_dir() {
        let qs_dir = home.join(".config/quickshell");
        if let Ok(entries) = std::fs::read_dir(qs_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && path.join("shell.qml").exists() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if seen.insert(name.to_string()) {
                            names.push(name.to_string());
                        }
                    }
                }
            }
        }
    }

    names
}

/// Set wallpaper in end-4 dots / Quickshell / AGS environment.
///
/// 1. Tries QuickShell IPC: `qs -c <config> ipc call wallpapers apply "<path>"`
///    and generic `qs ipc call wallpapers apply "<path>"`.
/// 2. Tries end-4 `switchwall.sh` script fallback across all known directories.
/// 3. Tries legacy AGS (end-4 dots v1/v2) if present.
pub async fn set(image_path: &str) -> Result<()> {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let config_names = discover_qs_config_names();

    // 1. Try QuickShell IPC across detected configuration names
    for name in &config_names {
        let status = Command::new("qs")
            .args(["-c", name, "ipc", "call", "wallpapers", "apply", image_path])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;

        if let Ok(s) = status {
            if s.success() {
                return Ok(());
            }
        }
    }

    // Generic QuickShell IPC fallback
    let generic_ipc = Command::new("qs")
        .args(["ipc", "call", "wallpapers", "apply", image_path])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await;

    if let Ok(s) = generic_ipc {
        if s.success() {
            return Ok(());
        }
    }

    // 2. Try end-4 switchwall.sh fallback
    for name in &config_names {
        let script = home.join(format!(".config/quickshell/{}/scripts/colors/switchwall.sh", name));
        if script.exists() {
            let status = Command::new("bash")
                .args([script.to_str().unwrap(), "--image", image_path])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await?;

            if status.success() {
                return Ok(());
            }
        }
    }

    // 3. Try legacy end-4 AGS switchwall.sh (v1/v2)
    let ags_script = home.join(".config/ags/scripts/color_generation/switchwall.sh");
    if ags_script.exists() {
        let status = Command::new("bash")
            .args([ags_script.to_str().unwrap(), "--image", image_path])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await?;

        if status.success() {
            return Ok(());
        }
    }

    anyhow::bail!("No active end-4 Quickshell / AGS wallpaper handler succeeded");
}
