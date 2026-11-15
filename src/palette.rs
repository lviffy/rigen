use std::collections::HashSet;
use std::path::Path;

use anyhow::{Context, Result};
use rand::seq::IndexedRandom;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::config::{expand_tilde, Config};

// ── Types ──

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    #[allow(dead_code)]
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Parse a 6-char hex string like `"1e1e2e"`.
    pub fn from_hex6(hex: &str) -> Option<Self> {
        if hex.len() < 6 {
            return None;
        }
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(Self { r, g, b })
    }

    /// Parse `#RRGGBB` or `#RRGGBBAA`.
    pub fn from_css_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim().trim_start_matches('#');
        if hex.len() >= 6 {
            Self::from_hex6(&hex[..6])
        } else {
            None
        }
    }

    /// Relative luminance (0.0–1.0).
    pub fn luminance(&self) -> f64 {
        let r = self.r as f64 / 255.0;
        let g = self.g as f64 / 255.0;
        let b = self.b as f64 / 255.0;
        0.2126 * r + 0.7152 * g + 0.0722 * b
    }

    /// HSV saturation (0.0–1.0).
    pub fn saturation(&self) -> f64 {
        let r = self.r as f64 / 255.0;
        let g = self.g as f64 / 255.0;
        let b = self.b as f64 / 255.0;
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        if max < 0.001 {
            0.0
        } else {
            (max - min) / max
        }
    }

    /// Euclidean distance in RGB space.
    pub fn distance(&self, other: &RgbColor) -> f64 {
        let dr = self.r as f64 - other.r as f64;
        let dg = self.g as f64 - other.g as f64;
        let db = self.b as f64 - other.b as f64;
        (dr * dr + dg * dg + db * db).sqrt()
    }

    pub fn to_hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    pub fn to_rgb_array(&self) -> [u8; 3] {
        [self.r, self.g, self.b]
    }
}

impl std::fmt::Display for RgbColor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

#[derive(Debug, Clone)]
pub struct ExtractedPalette {
    pub background: RgbColor,
    pub accents: Vec<RgbColor>,
    pub theme_name: Option<String>,
    pub is_free: bool,
}

impl ExtractedPalette {
    /// Palette for prompts where the AI model thinks and determines colors freely
    pub fn free_natural() -> Self {
        Self {
            background: RgbColor::new(26, 26, 36),
            accents: vec![],
            theme_name: Some("Natural (Prompt Inferred)".to_string()),
            is_free: true,
        }
    }
}

// ── TOML Theme Parser (for hyprsettings.toml) ──

/// A single `[[theme]]` entry from hyprsettings.toml.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct HyprSettingsTheme {
    name: Option<String>,
    #[serde(rename = "surface-0")]
    surface_0: Option<String>,
    #[serde(rename = "surface-1")]
    surface_1: Option<String>,
    #[serde(rename = "surface-2")]
    surface_2: Option<String>,
    accent: Option<String>,
    #[serde(rename = "accent-active")]
    accent_active: Option<String>,
    red: Option<String>,
    orange: Option<String>,
    yellow: Option<String>,
    green: Option<String>,
    teal: Option<String>,
    blue: Option<String>,
    pink: Option<String>,
    purple: Option<String>,
}

#[derive(Debug, Deserialize)]
struct HyprSettingsConfig {
    current_theme: Option<String>,
}

#[derive(Debug, Deserialize)]
struct HyprSettingsFile {
    config: Option<HyprSettingsConfig>,
    #[serde(default)]
    theme: Vec<HyprSettingsTheme>,
}

/// Extract palette from `hyprsettings.toml` by finding the active theme.
fn extract_from_hyprsettings(text: &str, max_accents: usize) -> Option<ExtractedPalette> {
    let settings: HyprSettingsFile = toml::from_str(text).ok()?;
    let current_name = settings.config.as_ref()?.current_theme.as_ref()?;
    let theme = settings
        .theme
        .iter()
        .find(|t| t.name.as_deref() == Some(current_name))?;

    let bg_hex = theme.surface_0.as_deref()?;
    let background = RgbColor::from_css_hex(bg_hex)?;

    let color_fields: Vec<Option<&String>> = vec![
        theme.accent.as_ref(),
        theme.accent_active.as_ref(),
        theme.blue.as_ref(),
        theme.purple.as_ref(),
        theme.red.as_ref(),
        theme.green.as_ref(),
        theme.pink.as_ref(),
        theme.teal.as_ref(),
        theme.orange.as_ref(),
        theme.yellow.as_ref(),
    ];

    let mut accents: Vec<RgbColor> = Vec::new();
    for field in color_fields.into_iter().flatten() {
        if let Some(color) = RgbColor::from_css_hex(field) {
            if color != background && !accents.iter().any(|c| c.distance(&color) < 25.0) {
                accents.push(color);
            }
        }
        if accents.len() >= max_accents {
            break;
        }
    }

    Some(ExtractedPalette {
        background,
        accents,
        theme_name: Some(current_name.clone()),
        is_free: false,
    })
}

// ── end-4 dots Material You colors.json Parser ──

/// Extract palette from end-4 dots generated colors.json.
fn extract_from_end4_colors_json(text: &str, max_accents: usize) -> Option<ExtractedPalette> {
    let json: serde_json::Value = serde_json::from_str(text).ok()?;
    let map = json.as_object()?;

    let bg_hex = map
        .get("background")
        .or_else(|| map.get("surface"))
        .and_then(|v| v.as_str())?;

    let background = RgbColor::from_css_hex(bg_hex)?;

    let candidate_keys = [
        "primary",
        "secondary",
        "tertiary",
        "primary_container",
        "secondary_container",
        "tertiary_container",
        "outline",
    ];

    let mut accents: Vec<RgbColor> = Vec::new();
    for key in candidate_keys {
        if let Some(val) = map.get(key).and_then(|v| v.as_str()) {
            if let Some(color) = RgbColor::from_css_hex(val) {
                if color != background && !accents.iter().any(|c| c.distance(&color) < 25.0) {
                    accents.push(color);
                }
            }
        }
        if accents.len() >= max_accents {
            break;
        }
    }

    Some(ExtractedPalette {
        background,
        accents,
        theme_name: Some("end-4 Material You".into()),
        is_free: false,
    })
}

// ── Regex Color Scraper (for colors.conf, hyprland.conf, etc.) ──

fn scrape_colors_regex(text: &str) -> Vec<RgbColor> {
    let mut colors = Vec::new();
    let mut seen = HashSet::new();

    let re_rgba_hex = Regex::new(r"rgba\s*\(\s*([0-9a-fA-F]{8})\s*\)").unwrap();
    for cap in re_rgba_hex.captures_iter(text) {
        if let Some(hex_match) = cap.get(1) {
            let s = hex_match.as_str();
            if let Some(c) = RgbColor::from_hex6(&s[0..6]) {
                if seen.insert(c) {
                    colors.push(c);
                }
            }
        }
    }

    let re_rgb_hex = Regex::new(r"rgb\s*\(\s*([0-9a-fA-F]{6})\s*\)").unwrap();
    for cap in re_rgb_hex.captures_iter(text) {
        if let Some(hex_match) = cap.get(1) {
            if let Some(c) = RgbColor::from_hex6(hex_match.as_str()) {
                if seen.insert(c) {
                    colors.push(c);
                }
            }
        }
    }

    let re_css_hex = Regex::new(r"#([0-9a-fA-F]{6})\b").unwrap();
    for cap in re_css_hex.captures_iter(text) {
        if let Some(hex_match) = cap.get(1) {
            if let Some(c) = RgbColor::from_hex6(hex_match.as_str()) {
                if seen.insert(c) {
                    colors.push(c);
                }
            }
        }
    }

    colors
}

fn classify_colors(colors: Vec<RgbColor>, max_accents: usize) -> Option<ExtractedPalette> {
    if colors.is_empty() {
        return None;
    }

    let background = *colors
        .iter()
        .min_by(|a, b| a.luminance().partial_cmp(&b.luminance()).unwrap())?;

    let mut accents: Vec<RgbColor> = colors
        .into_iter()
        .filter(|c| *c != background)
        .collect();

    accents.sort_by(|a, b| b.saturation().partial_cmp(&a.saturation()).unwrap());

    let mut deduped: Vec<RgbColor> = Vec::new();
    for color in accents {
        if !deduped.iter().any(|d| d.distance(&color) < 25.0) {
            deduped.push(color);
        }
        if deduped.len() >= max_accents {
            break;
        }
    }

    Some(ExtractedPalette {
        background,
        accents: deduped,
        theme_name: None,
        is_free: false,
    })
}

pub fn parse_explicit_colors(colors_str: &[String], max_accents: usize) -> Result<ExtractedPalette> {
    let colors: Vec<RgbColor> = colors_str
        .iter()
        .filter_map(|s| RgbColor::from_css_hex(s))
        .collect();

    anyhow::ensure!(!colors.is_empty(), "No valid hex colors found in --colors");

    let background = *colors
        .iter()
        .min_by(|a, b| a.luminance().partial_cmp(&b.luminance()).unwrap())
        .unwrap();

    let accents: Vec<RgbColor> = colors
        .iter()
        .filter(|c| **c != background)
        .copied()
        .take(max_accents)
        .collect();

    Ok(ExtractedPalette {
        background,
        accents,
        theme_name: Some("Custom CLI Colors".to_string()),
        is_free: false,
    })
}

/// Extract palette from a source file (TOML, JSON, or Hyprland conf).
pub fn extract_from_file(path: &Path, max_accents: usize) -> Result<ExtractedPalette> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read color source: {}", path.display()))?;

    // 1. Try TOML theme-aware parsing (hyprsettings.toml)
    if path.extension().map(|e| e == "toml").unwrap_or(false) {
        if let Some(palette) = extract_from_hyprsettings(&text, max_accents) {
            return Ok(palette);
        }
    }

    // 2. Try JSON Material You parsing (end-4 dots colors.json)
    if path.extension().map(|e| e == "json").unwrap_or(false) {
        if let Some(palette) = extract_from_end4_colors_json(&text, max_accents) {
            return Ok(palette);
        }
    }

    // 3. Fall back to regex scraping (colors.conf, hyprland.conf)
    let colors = scrape_colors_regex(&text);
    classify_colors(colors, max_accents)
        .context("No usable colors found in source file (need at least 1 color)")
}

/// Extract active desktop theme from dotfiles or config fallback.
pub fn resolve_desktop_palette(config: &Config) -> Result<ExtractedPalette> {
    let max = config.palette.max_accents;

    // 1. Config source_file
    let source_path = expand_tilde(&config.palette.source_file);
    if source_path.exists() {
        if let Ok(palette) = extract_from_file(&source_path, max) {
            return Ok(palette);
        }
    }

    // 2. Auto-detect end-4 dots palette sources
    let end4_sources = [
        "~/.config/hypr/hyprsettings.toml",
        "~/.local/state/quickshell/user/generated/colors.json",
        "~/.local/state/ags/user/generated/colors.json",
        "~/.config/hypr/hyprland/colors.conf",
        "~/.config/hypr/hyprland/colors.lua",
        "~/.config/illogical-impulse/config.json",
    ];

    for candidate in &end4_sources {
        let p = expand_tilde(candidate);
        if p.exists() {
            if let Ok(palette) = extract_from_file(&p, max) {
                return Ok(palette);
            }
        }
    }

    // 3. Fallback palette from config
    let background = RgbColor::from_css_hex(&config.palette.fallback_bg)
        .context("Invalid fallback_bg hex in config")?;
    let accents: Vec<RgbColor> = config
        .palette
        .fallback_accents
        .iter()
        .filter_map(|s| RgbColor::from_css_hex(s))
        .take(max)
        .collect();

    Ok(ExtractedPalette {
        background,
        accents,
        theme_name: Some("Desktop Fallback".to_string()),
        is_free: false,
    })
}

/// Pick a random vibrant, aesthetic color scheme.
pub fn random_aesthetic_palette(max_accents: usize) -> ExtractedPalette {
    let mut rng = rand::rng();
    let themes: &[(&str, &str, &[&str])] = &[
        ("Cyberpunk Neon", "#090912", &["#ff007f", "#00f0ff", "#ffe600", "#a855f7"]),
        ("Synthwave Sunset", "#160728", &["#ff2a85", "#ff7300", "#00f5d4", "#7b2cbf"]),
        ("Emerald Rainforest", "#061810", &["#2d6a4f", "#52b788", "#74c69d", "#d8f3dc"]),
        ("Solar Flare", "#160c10", &["#ea580c", "#f59e0b", "#e11d48", "#fbbf24"]),
        ("Tokyo Midnight", "#0a0e18", &["#8b5cf6", "#06b6d4", "#f43f5e", "#3b82f6"]),
        ("Sakura Bloom", "#180d1b", &["#f472b6", "#c084fc", "#fda4af", "#fbcfe8"]),
        ("Nordic Glacier", "#081017", &["#38bdf8", "#0284c7", "#7dd3fc", "#e0f2fe"]),
        ("Autumn Amber", "#180e07", &["#c2410c", "#d97706", "#b45309", "#f59e0b"]),
        ("Crimson Vampire", "#0a0305", &["#991b1b", "#dc2626", "#ef4444", "#f1f5f9"]),
        ("Deep Ocean Abyss", "#010912", &["#0077b6", "#00b4d8", "#48cae4", "#90e0ef"]),
        ("Matrix Phosphor", "#030c05", &["#22c55e", "#16a34a", "#4ade80", "#86efac"]),
        ("Pastel Dreamscape", "#110e1c", &["#f9a8d4", "#a7f3d0", "#fed7aa", "#c4b5fd"]),
        ("Golden Hour Dunes", "#1a130e", &["#f59e0b", "#fb923c", "#fcd34d", "#fdba74"]),
        ("Amethyst Nebula", "#0c0615", &["#a855f7", "#c084fc", "#ec4899", "#e9d5ff"]),
        ("Retro Vaporwave", "#120724", &["#ff71ce", "#01cdfe", "#05ffa1", "#b967ff"]),
        ("Monochrome Noir", "#09090b", &["#ffffff", "#a1a1aa", "#71717a", "#3f3f46"]),
        ("Volcanic Magma", "#120404", &["#ef4444", "#f97316", "#facc15", "#b91c1c"]),
        ("Bioluminescent Lagoon", "#011015", &["#06d6a0", "#118ab2", "#ffd166", "#ef476f"]),
        ("Lavender Twilight", "#0f0c1a", &["#a78bfa", "#c4b5fd", "#818cf8", "#ddd6fe"]),
        ("Desert Sunset", "#180c12", &["#f43f5e", "#fb923c", "#fbbf24", "#be185d"]),
        ("Cyber Ice", "#040d15", &["#00f0ff", "#7000ff", "#ff0070", "#00ffa3"]),
        ("Electric Violet", "#0c0318", &["#7c3aed", "#a855f7", "#38bdf8", "#f43f5e"]),
        ("Alpine Aurora", "#05120a", &["#10b981", "#3b82f6", "#f59e0b", "#6ee7b7"]),
        ("Starry Void", "#04040d", &["#6366f1", "#a855f7", "#38bdf8", "#fde047"]),
    ];

    let (name, bg_hex, accents_hexes) = themes.choose(&mut rng).unwrap();
    let background = RgbColor::from_css_hex(bg_hex).unwrap();
    let accents = accents_hexes
        .iter()
        .take(max_accents)
        .filter_map(|h| RgbColor::from_css_hex(h))
        .collect();

    ExtractedPalette {
        background,
        accents,
        theme_name: Some(format!("{} (Random Lucky)", name)),
        is_free: false,
    }
}

/// Extract colors mentioned directly in the user prompt (either hex codes or color words).
pub fn extract_colors_from_prompt(prompt: &str, max_accents: usize) -> Option<ExtractedPalette> {
    // 1. Check for CSS hex codes in prompt: e.g. #ff0055 or #00f0ff
    let re_hex = Regex::new(r"#([0-9a-fA-F]{6})\b").unwrap();
    let mut hex_colors = Vec::new();
    let mut seen = HashSet::new();

    for cap in re_hex.captures_iter(prompt) {
        if let Some(m) = cap.get(1) {
            if let Some(c) = RgbColor::from_hex6(m.as_str()) {
                if seen.insert(c) {
                    hex_colors.push(c);
                }
            }
        }
    }

    if !hex_colors.is_empty() {
        if hex_colors.len() == 1 {
            let accent = hex_colors[0];
            let background = RgbColor::new(12, 12, 18);
            return Some(ExtractedPalette {
                background,
                accents: vec![accent],
                theme_name: Some(format!("Prompt Hex ({})", accent.to_hex())),
                is_free: false,
            });
        } else {
            let background = *hex_colors
                .iter()
                .min_by(|a, b| a.luminance().partial_cmp(&b.luminance()).unwrap())
                .unwrap();
            let accents: Vec<RgbColor> = hex_colors
                .into_iter()
                .filter(|c| *c != background)
                .take(max_accents)
                .collect();
            return Some(ExtractedPalette {
                background,
                accents,
                theme_name: Some("Prompt Hex Colors".to_string()),
                is_free: false,
            });
        }
    }

    // 2. Check for named colors
    const NAMED_COLORS: &[(&str, &str, [u8; 3])] = &[
        ("hot pink", "Hot Pink", [255, 20, 147]),
        ("neon pink", "Neon Pink", [255, 16, 128]),
        ("pastel pink", "Pastel Pink", [255, 182, 193]),
        ("baby pink", "Baby Pink", [244, 194, 194]),
        ("cherry blossom", "Cherry Blossom", [255, 183, 197]),
        ("rose gold", "Rose Gold", [183, 110, 121]),
        ("neon green", "Neon Green", [57, 255, 20]),
        ("forest green", "Forest Green", [34, 139, 34]),
        ("emerald green", "Emerald Green", [46, 204, 113]),
        ("mint green", "Mint Green", [152, 255, 152]),
        ("electric blue", "Electric Blue", [0, 212, 255]),
        ("neon blue", "Neon Blue", [31, 81, 255]),
        ("sky blue", "Sky Blue", [135, 206, 235]),
        ("navy blue", "Navy Blue", [10, 25, 47]),
        ("deep blue", "Deep Blue", [0, 20, 60]),
        ("dark blue", "Dark Blue", [10, 25, 47]),
        ("royal blue", "Royal Blue", [65, 105, 225]),
        ("neon yellow", "Neon Yellow", [255, 255, 51]),
        ("golden yellow", "Golden Yellow", [255, 223, 0]),
        ("pastel purple", "Pastel Purple", [203, 166, 247]),
        ("deep purple", "Deep Purple", [48, 15, 60]),
        ("dark purple", "Dark Purple", [48, 20, 52]),
        ("burnt orange", "Burnt Orange", [204, 85, 0]),
        ("blood red", "Blood Red", [138, 3, 3]),
        ("crimson red", "Crimson", [185, 28, 28]),
        ("black and white", "Monochrome", [240, 240, 245]),
        ("crimson", "Crimson", [185, 28, 28]),
        ("scarlet", "Scarlet", [220, 38, 38]),
        ("ruby", "Ruby", [225, 29, 72]),
        ("magenta", "Magenta", [217, 70, 239]),
        ("fuchsia", "Fuchsia", [192, 38, 211]),
        ("pink", "Pink", [236, 72, 153]),
        ("rose", "Rose", [244, 63, 94]),
        ("coral", "Coral", [249, 115, 22]),
        ("orange", "Orange", [234, 88, 12]),
        ("amber", "Amber", [217, 119, 6]),
        ("gold", "Gold", [245, 158, 11]),
        ("golden", "Gold", [245, 158, 11]),
        ("yellow", "Yellow", [234, 179, 8]),
        ("lime", "Lime", [132, 204, 22]),
        ("emerald", "Emerald", [16, 185, 129]),
        ("mint", "Mint", [110, 231, 183]),
        ("teal", "Teal", [13, 148, 136]),
        ("cyan", "Cyan", [6, 182, 212]),
        ("turquoise", "Turquoise", [20, 184, 166]),
        ("aqua", "Aqua", [0, 255, 255]),
        ("sapphire", "Sapphire", [15, 82, 186]),
        ("indigo", "Indigo", [99, 102, 241]),
        ("violet", "Violet", [124, 58, 237]),
        ("purple", "Purple", [147, 51, 234]),
        ("lavender", "Lavender", [192, 132, 252]),
        ("amethyst", "Amethyst", [153, 102, 204]),
        ("silver", "Silver", [203, 213, 225]),
        ("bronze", "Bronze", [205, 127, 50]),
        ("copper", "Copper", [184, 115, 51]),
        ("red", "Red", [239, 68, 68]),
        ("green", "Green", [34, 197, 94]),
        ("blue", "Blue", [59, 130, 246]),
        ("navy", "Navy", [15, 23, 42]),
        ("white", "White", [248, 250, 252]),
        ("black", "Black", [10, 10, 15]),
        ("charcoal", "Charcoal", [31, 41, 55]),
        ("peach", "Peach", [251, 146, 60]),
    ];

    let lower_prompt = prompt.to_lowercase();
    let mut matched_names = Vec::new();
    let mut matched_colors = Vec::new();
    let mut seen_colors = HashSet::new();
    let mut matched_spans: Vec<(usize, usize)> = Vec::new();

    for (needle, label, rgb) in NAMED_COLORS {
        let pattern = format!(r"\b{}\b", regex::escape(needle));
        if let Ok(re) = Regex::new(&pattern) {
            for m in re.find_iter(&lower_prompt) {
                let start = m.start();
                let end = m.end();

                let overlaps = matched_spans.iter().any(|(s, e)| start < *e && end > *s);
                if !overlaps {
                    matched_spans.push((start, end));
                    let color = RgbColor::new(rgb[0], rgb[1], rgb[2]);
                    if seen_colors.insert(color) {
                        matched_colors.push(color);
                        if !matched_names.contains(&label.to_string()) {
                            matched_names.push(label.to_string());
                        }
                    }
                }
            }
        }
    }

    if matched_colors.is_empty() {
        return None;
    }

    let dark_candidate = matched_colors
        .iter()
        .filter(|c| c.luminance() < 0.10)
        .min_by(|a, b| a.luminance().partial_cmp(&b.luminance()).unwrap())
        .copied();

    let (background, accents) = if let Some(bg) = dark_candidate {
        let acc: Vec<RgbColor> = matched_colors
            .into_iter()
            .filter(|c| *c != bg)
            .take(max_accents)
            .collect();
        (bg, acc)
    } else {
        let bg = RgbColor::new(10, 11, 16);
        let acc: Vec<RgbColor> = matched_colors.into_iter().take(max_accents).collect();
        (bg, acc)
    };

    let theme_label = format!("Prompt: {}", matched_names.join(", "));
    Some(ExtractedPalette {
        background,
        accents,
        theme_name: Some(theme_label),
        is_free: false,
    })
}

/// Extract dominant colors from an image on disk (used after free generation to update metadata/preview).
pub fn sample_image_palette(image_path: &Path, max_accents: usize) -> Option<ExtractedPalette> {
    let img = image::open(image_path).ok()?.to_rgb8();
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return None;
    }

    let step_x = (w / 16).max(1);
    let step_y = (h / 16).max(1);
    let mut sampled_colors = Vec::new();

    for y in (0..h).step_by(step_y as usize) {
        for x in (0..w).step_by(step_x as usize) {
            let p = img.get_pixel(x, y);
            sampled_colors.push(RgbColor::new(p[0], p[1], p[2]));
        }
    }

    let mut palette = classify_colors(sampled_colors, max_accents)?;
    palette.theme_name = Some("Generated Image Colors".to_string());
    Some(palette)
}

/// Resolve palette with dynamic prompt inference and random lucky palettes:
/// 1. Explicit --colors
/// 2. Explicit --source
/// 3. --random-palette OR (--lucky without --rice) -> Random Aesthetic Palette
/// 4. --rice OR config.palette.mode == "rice" -> Active Desktop Rice Theme
/// 5. Prompt colors (hex codes or named color words in the prompt text)
/// 6. Default -> Natural (model thinks freely from the prompt without color clamping)
pub fn resolve_palette(
    explicit_colors: &Option<Vec<String>>,
    source_override: &Option<String>,
    prompt: &str,
    is_lucky: bool,
    force_rice: bool,
    random_palette: bool,
    config: &Config,
) -> Result<ExtractedPalette> {
    let max = config.palette.max_accents;

    // 1. Explicit --colors
    if let Some(colors) = explicit_colors {
        return parse_explicit_colors(colors, max);
    }

    // 2. --source override
    if let Some(src) = source_override {
        let path = expand_tilde(src);
        return extract_from_file(&path, max);
    }

    // 3. Random palette requested OR --lucky (unless user explicitly passed --rice)
    if random_palette || (is_lucky && !force_rice) {
        return Ok(random_aesthetic_palette(max));
    }

    // 4. Explicit --rice OR config configured to always lock to desktop rice
    if force_rice || config.palette.mode == "rice" {
        return resolve_desktop_palette(config);
    }

    // 5. Inferred from colors in prompt
    if let Some(prompt_palette) = extract_colors_from_prompt(prompt, max) {
        return Ok(prompt_palette);
    }

    // 6. Natural / Free: let the model think freely from the prompt!
    Ok(ExtractedPalette::free_natural())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_parsing() {
        let c = RgbColor::from_css_hex("#1e1e2e").unwrap();
        assert_eq!(c, RgbColor::new(30, 30, 46));

        let c = RgbColor::from_css_hex("#89b4fa").unwrap();
        assert_eq!(c, RgbColor::new(137, 180, 250));
    }

    #[test]
    fn test_rgba_scraping() {
        let text = r#"
            col.active_border = rgba(9f8c8d77)
            background_color = rgba(1a1112FF)
            col.text = rgba(f0dedeFF)
        "#;
        let colors = scrape_colors_regex(text);
        assert_eq!(colors.len(), 3);
        assert_eq!(colors[0], RgbColor::new(0x9f, 0x8c, 0x8d));
        assert_eq!(colors[1], RgbColor::new(0x1a, 0x11, 0x12));
    }

    #[test]
    fn test_rgb_scraping() {
        let text = "hyprbars-button = rgb(f0dede), 13";
        let colors = scrape_colors_regex(text);
        assert_eq!(colors.len(), 1);
        assert_eq!(colors[0], RgbColor::new(0xf0, 0xde, 0xde));
    }

    #[test]
    fn test_luminance_picks_darkest() {
        let colors = vec![
            RgbColor::new(30, 30, 46),   // #1e1e2e – darkest
            RgbColor::new(137, 180, 250), // #89b4fa
            RgbColor::new(203, 166, 247), // #cba6f7
        ];
        let palette = classify_colors(colors, 4).unwrap();
        assert_eq!(palette.background, RgbColor::new(30, 30, 46));
    }

    #[test]
    fn test_hyprsettings_toml() {
        let toml_text = r##"
[config]
current_theme = "Catppuccin Mocha"

[[theme]]
name = "Catppuccin Mocha"
surface-0 = "#1e1e2e"
accent = "#89b4fa"
red = "#f38ba8"
green = "#a6e3a1"
blue = "#89b4fa"
purple = "#cba6f7"
"##;
        let palette = extract_from_hyprsettings(toml_text, 4).unwrap();
        assert_eq!(palette.background, RgbColor::new(0x1e, 0x1e, 0x2e));
        assert_eq!(palette.theme_name.as_deref(), Some("Catppuccin Mocha"));
        assert!(!palette.accents.is_empty());
    }

    #[test]
    fn test_end4_colors_json() {
        let json_text = r##"
{
  "background": "#131314",
  "primary": "#bbc8d2",
  "secondary": "#a2c9e8",
  "tertiary": "#d1b4f4"
}
"##;
        let palette = extract_from_end4_colors_json(json_text, 4).unwrap();
        assert_eq!(palette.background, RgbColor::new(0x13, 0x13, 0x14));
        assert_eq!(palette.theme_name.as_deref(), Some("end-4 Material You"));
        assert_eq!(palette.accents.len(), 3);
        assert_eq!(palette.accents[0], RgbColor::new(0xbb, 0xc8, 0xd2));
    }

    #[test]
    fn test_extract_colors_from_prompt_hex() {
        let p = "Cyberpunk street with #ff007f neon and #00f0ff reflections";
        let palette = extract_colors_from_prompt(p, 4).unwrap();
        assert!(!palette.is_free);
        assert_eq!(palette.accents.len(), 1);
        assert!(palette.accents.iter().any(|c| c.to_hex() == "#00f0ff" || c.to_hex() == "#ff007f"));
    }

    #[test]
    fn test_extract_colors_from_prompt_named() {
        let p = "Samurai in crimson red and gold armor under cherry blossoms";
        let palette = extract_colors_from_prompt(p, 4).unwrap();
        assert!(!palette.is_free);
        assert!(!palette.accents.is_empty());
        assert!(palette.theme_name.as_deref().unwrap().contains("Crimson"));
    }

    #[test]
    fn test_extract_colors_from_prompt_free() {
        let p = "Cosmic floating astronaut in zero gravity near a distant galaxy";
        let palette = extract_colors_from_prompt(p, 4);
        assert!(palette.is_none());
    }

    #[test]
    fn test_random_aesthetic_palette() {
        let p = random_aesthetic_palette(4);
        assert!(!p.is_free);
        assert!(!p.accents.is_empty());
        assert!(p.theme_name.as_deref().unwrap().contains("Random Lucky"));
    }
}
