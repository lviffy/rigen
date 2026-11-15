pub mod openrouter;
pub mod recraft;

use anyhow::Result;
use std::path::Path;

use crate::config::Config;
use crate::palette::ExtractedPalette;

/// Generate an image and save it to `output_path`. Returns the path written.
pub async fn generate(
    config: &Config,
    prompt: &str,
    palette: &ExtractedPalette,
    output_path: &Path,
) -> Result<()> {
    match config.api.provider.as_str() {
        "recraft" => recraft::generate(config, prompt, palette, output_path).await,
        "openrouter" | _ => openrouter::generate(config, prompt, palette, output_path).await,
    }
}

