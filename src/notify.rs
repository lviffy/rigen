use std::process::Stdio;
use tokio::process::Command;

/// Send desktop notification with the generated image as an icon.
pub async fn send(image_path: &str, prompt: &str) {
    let summary = "🖼 New Wallpaper Applied";
    let body = if prompt.len() > 80 {
        format!("{}...", &prompt[..77])
    } else {
        prompt.to_string()
    };

    let _ = Command::new("notify-send")
        .args([
            "-a",
            "rice-gen",
            "-i",
            image_path,
            summary,
            &body,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await;
}

