use std::path::Path;

use anyhow::{bail, Context, Result};
use futures_util::StreamExt;
use serde_json::json;
use tokio::io::AsyncWriteExt;

use crate::config::Config;
use crate::palette::ExtractedPalette;

/// Generate via native Recraft REST API.
///
/// POST https://external.api.recraft.ai/v1/images/generations
/// Uses `controls.colors` and `controls.background_color` for strict palette adherence.
/// Response contains `data[0].url` which we stream-download to disk.
pub async fn generate(
    config: &Config,
    prompt: &str,
    palette: &ExtractedPalette,
    output_path: &Path,
) -> Result<()> {
    let api_key = config.api_key()?;

    let body = if palette.is_free {
        // Model thinks freely from the prompt!
        json!({
            "model": config.api.model,
            "prompt": format!("{}, desktop wallpaper", prompt),
            "size": config.api.aspect_ratio,
            "response_format": "url",
        })
    } else {
        // Build color controls
        let accent_colors: Vec<serde_json::Value> = palette
            .accents
            .iter()
            .map(|c| json!({ "rgb": [c.r, c.g, c.b] }))
            .collect();

        let bg = &palette.background;
        json!({
            "model": config.api.model,
            "prompt": format!("{}, desktop wallpaper", prompt),
            "size": config.api.aspect_ratio,
            "response_format": "url",
            "controls": {
                "background_color": { "rgb": [bg.r, bg.g, bg.b] },
                "colors": accent_colors,
            }
        })
    };

    let client = reqwest::Client::new();
    let resp = client
        .post("https://external.api.recraft.ai/v1/images/generations")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .context("Failed to connect to Recraft API")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if status.as_u16() == 429 {
            bail!("Rate limited by Recraft API. Try again shortly.\n{}", text);
        }
        bail!("Recraft API error {}: {}", status, text);
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .context("Failed to parse Recraft API response")?;

    let image_url = json["data"][0]["url"]
        .as_str()
        .context("No image URL in Recraft response")?;

    // Stream-download the image to disk
    let img_resp = client
        .get(image_url)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .context("Failed to download generated image")?;

    if !img_resp.status().is_success() {
        bail!(
            "Failed to download image: HTTP {}",
            img_resp.status()
        );
    }

    // Ensure parent directory exists
    if let Some(parent) = output_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let mut file = tokio::fs::File::create(output_path)
        .await
        .with_context(|| format!("Failed to create {}", output_path.display()))?;

    let mut stream = img_resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let bytes = chunk.context("Error reading image stream")?;
        file.write_all(&bytes).await?;
    }

    file.flush().await?;
    Ok(())
}

