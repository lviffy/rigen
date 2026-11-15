use std::path::Path;

use anyhow::{bail, Context, Result};
use base64::Engine;
use futures_util::StreamExt;
use serde_json::json;
use tokio::io::AsyncWriteExt;

use crate::config::Config;
use crate::palette::ExtractedPalette;

/// Generate via OpenRouter Image API.
///
/// POST https://openrouter.ai/api/v1/images
/// Passes `image_config` with aspect ratio and RGB colors,
/// plus palette enrichment in the prompt for universal adherence.
pub async fn generate(
    config: &Config,
    prompt: &str,
    palette: &ExtractedPalette,
    output_path: &Path,
) -> Result<()> {
    let api_key = config.api_key()?;

    let body = if palette.is_free {
        // Model thinks freely from the prompt! No artificial color clamping
        let p = format!("{}, desktop wallpaper", prompt);
        json!({
            "model": config.api.model,
            "prompt": p,
            "image_config": {
                "aspect_ratio": config.api.aspect_ratio,
            }
        })
    } else {
        // Enforce RGB color constraints from prompt or selected palette
        let rgb_colors: Vec<[u8; 3]> = palette.accents.iter().map(|c| c.to_rgb_array()).collect();
        let bg = &palette.background;

        let accents_str = palette
            .accents
            .iter()
            .map(|c| c.to_hex())
            .collect::<Vec<_>>()
            .join(", ");

        let p = format!(
            "{}, desktop wallpaper, dominant background {}, accents in {}",
            prompt,
            bg.to_hex(),
            accents_str
        );

        json!({
            "model": config.api.model,
            "prompt": p,
            "image_config": {
                "aspect_ratio": config.api.aspect_ratio,
                "rgb_colors": rgb_colors,
                "background_rgb_color": [bg.r, bg.g, bg.b],
            }
        })
    };

    let client = reqwest::Client::new();
    let resp = client
        .post("https://openrouter.ai/api/v1/images")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .timeout(std::time::Duration::from_secs(45))
        .send()
        .await
        .context("Failed to connect to OpenRouter API")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if status.as_u16() == 429 {
            bail!("Rate limited by OpenRouter. Try again shortly.\n{}", text);
        }
        if text.contains("Insufficient credits") || status.as_u16() == 402 {
            bail!(
                "Insufficient OpenRouter credits.\nTop up your balance at https://openrouter.ai/settings/credits ($0.005/image)"
            );
        }
        bail!("OpenRouter API error {}: {}", status, text);
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .context("Failed to parse OpenRouter response")?;

    // Ensure parent directory exists
    if let Some(parent) = output_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    // OpenRouter can return either base64-encoded image data or a direct URL
    if let Some(b64_data) = json["data"][0]["b64_json"].as_str() {
        let image_bytes = base64::engine::general_purpose::STANDARD
            .decode(b64_data)
            .context("Failed to decode base64 image data")?;

        let mut file = tokio::fs::File::create(output_path)
            .await
            .with_context(|| format!("Failed to create {}", output_path.display()))?;

        file.write_all(&image_bytes).await?;
        file.flush().await?;
    } else if let Some(image_url) = json["data"][0]["url"].as_str() {
        let img_resp = client
            .get(image_url)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .context("Failed to download generated image from URL")?;

        let mut file = tokio::fs::File::create(output_path)
            .await
            .with_context(|| format!("Failed to create {}", output_path.display()))?;

        let mut stream = img_resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let bytes = chunk.context("Error reading image stream")?;
            file.write_all(&bytes).await?;
        }
        file.flush().await?;
    } else {
        bail!("No recognizable image data (b64_json or url) in OpenRouter response");
    }

    Ok(())
}
