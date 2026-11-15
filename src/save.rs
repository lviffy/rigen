use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Local;
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::palette::ExtractedPalette;

#[derive(Debug, Serialize, Deserialize)]
pub struct WallpaperMetadata {
    pub prompt: String,
    pub model: String,
    pub provider: String,
    pub background: String,
    pub accents: Vec<String>,
    pub created_at: String,
    pub file_path: String,
}

/// Write metadata sidecar for a freshly generated wallpaper.
pub fn record_generation_meta(
    meta_path: &Path,
    image_path: &Path,
    prompt: &str,
    palette: &ExtractedPalette,
    config: &Config,
) -> Result<()> {
    let meta = WallpaperMetadata {
        prompt: prompt.to_string(),
        model: config.api.model.clone(),
        provider: config.api.provider.clone(),
        background: palette.background.to_hex(),
        accents: palette.accents.iter().map(|c| c.to_hex()).collect(),
        created_at: Local::now().to_rfc3339(),
        file_path: image_path.to_string_lossy().to_string(),
    };

    let json_text = serde_json::to_string_pretty(&meta)?;
    std::fs::write(meta_path, json_text)?;

    Ok(())
}

/// Read the latest metadata from cache.
pub fn read_latest_meta(cache_dir: &Path) -> Option<WallpaperMetadata> {
    let meta_path = cache_dir.join("latest.json");
    if meta_path.exists() {
        let text = std::fs::read_to_string(&meta_path).ok()?;
        serde_json::from_str(&text).ok()
    } else {
        None
    }
}

/// Save a freshly generated wallpaper directly into save_dir (e.g. ~/Pictures/Wallpapers).
#[allow(dead_code)]
pub fn save_generated(
    config: &Config,
    source_image: &Path,
    prompt: &str,
    palette: &ExtractedPalette,
) -> Result<PathBuf> {
    let save_dir = config.save_dir();
    std::fs::create_dir_all(&save_dir)?;

    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let dest_image = save_dir.join(format!("rice-gen_{}.png", timestamp));
    let dest_meta = save_dir.join(format!("rice-gen_{}.json", timestamp));

    std::fs::copy(source_image, &dest_image)
        .with_context(|| format!("Failed to copy image to {}", dest_image.display()))?;

    let meta = WallpaperMetadata {
        prompt: prompt.to_string(),
        model: config.api.model.clone(),
        provider: config.api.provider.clone(),
        background: palette.background.to_hex(),
        accents: palette.accents.iter().map(|c| c.to_hex()).collect(),
        created_at: Local::now().to_rfc3339(),
        file_path: dest_image.to_string_lossy().to_string(),
    };

    let json_text = serde_json::to_string_pretty(&meta)?;
    let _ = std::fs::write(&dest_meta, json_text);

    Ok(dest_image)
}

/// Save the latest generated wallpaper to permanent storage with a custom name.
pub fn save_current(config: &Config, custom_name: Option<String>) -> Result<PathBuf> {
    let cache_dir = config.cache_dir();
    let save_dir = config.save_dir();
    std::fs::create_dir_all(&save_dir)?;

    let source_image = if cache_dir.join("latest.png").exists() {
        cache_dir.join("latest.png")
    } else {
        cache_dir.join("latest.webp")
    };

    anyhow::ensure!(
        source_image.exists(),
        "No generated wallpaper found in cache ({}) to save. Generate one first!",
        source_image.display()
    );

    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let ext = source_image
        .extension()
        .and_then(|e: &std::ffi::OsStr| e.to_str())
        .unwrap_or("png");

    let base_name = if let Some(name) = custom_name {
        let sanitized: String = name
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect();
        format!("{}_{}", sanitized, timestamp)
    } else {
        format!("wall_{}", timestamp)
    };

    let dest_image = save_dir.join(format!("{}.{}", base_name, ext));
    let dest_meta = save_dir.join(format!("{}.json", base_name));

    // Copy image
    std::fs::copy(&source_image, &dest_image)
        .with_context(|| format!("Failed to copy image to {}", dest_image.display()))?;

    // Copy or create metadata
    let mut meta = read_latest_meta(&cache_dir).unwrap_or_else(|| WallpaperMetadata {
        prompt: "unknown".into(),
        model: config.api.model.clone(),
        provider: config.api.provider.clone(),
        background: "".into(),
        accents: vec![],
        created_at: Local::now().to_rfc3339(),
        file_path: dest_image.to_string_lossy().to_string(),
    });

    meta.file_path = dest_image.to_string_lossy().to_string();
    let json_text = serde_json::to_string_pretty(&meta)?;
    std::fs::write(&dest_meta, json_text)?;

    Ok(dest_image)
}

/// List recent saved or cached wallpapers.
pub fn list_history(config: &Config) -> Result<()> {
    let save_dir = config.save_dir();
    if !save_dir.exists() {
        println!("No saved wallpapers directory at {}", save_dir.display());
        return Ok(());
    }

    println!("\n  ── Saved Wallpapers ({}) ──", save_dir.display());
    let mut count = 0;

    if let Ok(entries) = std::fs::read_dir(&save_dir) {
        let mut metas: Vec<WallpaperMetadata> = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(text) = std::fs::read_to_string(&path) {
                    if let Ok(m) = serde_json::from_str::<WallpaperMetadata>(&text) {
                        metas.push(m);
                    }
                }
            }
        }

        metas.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        for m in metas.iter().take(10) {
            count += 1;
            println!(
                "  [{}] {} | \"{}\"",
                count,
                &m.created_at[..19.min(m.created_at.len())],
                m.prompt
            );
            println!("      File: {}", m.file_path);
            println!("      Palette: BG {} | Accents: {}", m.background, m.accents.join(", "));
        }
    }

    if count == 0 {
        println!("  No saved wallpapers yet.");
    }
    println!();

    Ok(())
}
