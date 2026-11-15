use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::Deserialize;

/// Expand `~` to the user's home directory.
pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_api")]
    pub api: ApiConfig,
    #[serde(default = "default_palette")]
    pub palette: PaletteConfig,
    #[serde(default = "default_wallpaper")]
    pub wallpaper: WallpaperConfig,
    #[serde(default)]
    pub lucky: LuckyConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiConfig {
    #[serde(default = "default_provider")]
    pub provider: String,
    pub api_key: Option<String>,
    #[serde(default = "default_api_key_env")]
    pub api_key_env: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_aspect_ratio")]
    pub aspect_ratio: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PaletteConfig {
    #[serde(default = "default_palette_mode")]
    pub mode: String,
    #[serde(default = "default_source_file")]
    pub source_file: String,
    #[serde(default = "default_max_accents")]
    pub max_accents: usize,
    #[serde(default = "default_fallback_bg")]
    pub fallback_bg: String,
    #[serde(default = "default_fallback_accents")]
    pub fallback_accents: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WallpaperConfig {
    #[serde(default = "default_backend")]
    pub backend: String,
    #[serde(default = "default_cache_dir")]
    pub cache_dir: String,
    #[serde(default = "default_save_dir")]
    pub save_dir: String,
    #[serde(default)]
    pub sync_matugen: bool,
    pub post_hook: Option<String>,
    #[serde(default)]
    pub mpvpaper: MpvpaperConfig,
    #[serde(default)]
    pub swww: SwwwConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MpvpaperConfig {
    #[serde(default = "default_mpvpaper_args")]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SwwwConfig {
    #[serde(default = "default_swww_args")]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LuckyConfig {
    #[serde(default = "default_prompts_file")]
    pub prompts_file: String,
}

// ── Defaults ──

fn default_api() -> ApiConfig {
    ApiConfig {
        provider: default_provider(),
        api_key: None,
        api_key_env: default_api_key_env(),
        model: default_model(),
        aspect_ratio: default_aspect_ratio(),
    }
}
fn default_palette_mode() -> String { "auto".into() }
fn default_palette() -> PaletteConfig {
    PaletteConfig {
        mode: default_palette_mode(),
        source_file: default_source_file(),
        max_accents: default_max_accents(),
        fallback_bg: default_fallback_bg(),
        fallback_accents: default_fallback_accents(),
    }
}
fn default_wallpaper() -> WallpaperConfig {
    WallpaperConfig {
        backend: default_backend(),
        cache_dir: default_cache_dir(),
        save_dir: default_save_dir(),
        sync_matugen: false,
        post_hook: None,
        mpvpaper: MpvpaperConfig::default(),
        swww: SwwwConfig::default(),
    }
}

fn default_provider() -> String { "openrouter".into() }
fn default_api_key_env() -> String { "OPENROUTER_API_KEY".into() }
fn default_model() -> String { "recraft/recraft-v4.1-flash-v2:free".into() }
fn default_aspect_ratio() -> String { "16:9".into() }
fn default_source_file() -> String { "~/.config/hypr/hyprsettings.toml".into() }
fn default_max_accents() -> usize { 4 }
fn default_fallback_bg() -> String { "#1e1e2e".into() }
fn default_fallback_accents() -> Vec<String> {
    vec![
        "#89b4fa".into(),
        "#cba6f7".into(),
        "#f38ba8".into(),
        "#a6e3a1".into(),
    ]
}
fn default_backend() -> String { "auto".into() }
fn default_cache_dir() -> String { "~/.cache/rice-gen".into() }
fn default_save_dir() -> String { "~/Pictures/Wallpapers".into() }
fn default_prompts_file() -> String { "~/.config/rice-gen/prompts.toml".into() }

fn default_mpvpaper_args() -> Vec<String> {
    vec![
        "-f".into(),
        "--auto-pause".into(),
        "-o".into(),
        "no-audio loop".into(),
    ]
}
fn default_swww_args() -> Vec<String> {
    vec![
        "--transition-type".into(),
        "wipe".into(),
        "--transition-angle".into(),
        "30".into(),
        "--transition-fps".into(),
        "120".into(),
    ]
}

impl Default for MpvpaperConfig {
    fn default() -> Self {
        Self { args: default_mpvpaper_args() }
    }
}
impl Default for SwwwConfig {
    fn default() -> Self {
        Self { args: default_swww_args() }
    }
}
impl Default for LuckyConfig {
    fn default() -> Self {
        Self { prompts_file: default_prompts_file() }
    }
}

pub const DEFAULT_CONFIG_TEMPLATE: &str = r##"# rice-gen configuration
# Location: ~/.config/rice-gen/config.toml

[api]
# Provider: "openrouter" or "recraft"
provider = "openrouter"

# OpenRouter Model Slug (free tier):
model = "recraft/recraft-v4.1-flash-v2:free"

# Environment variable name holding your API key
api_key_env = "OPENROUTER_API_KEY"

# Optional direct fallback API key (prefer environment variable for security)
# api_key = "sk-or-v1-..."

# Default aspect ratio ("16:9", "16:10", "21:9", "4:3", "1:1")
aspect_ratio = "16:9"

[palette]
# Palette mode:
# - "auto" (default): Model thinks freely from prompt, extracts colors if mentioned, randomizes for --lucky.
# - "rice": Always lock wallpapers to your active Hyprland desktop theme.
mode = "auto"

# Dotfile to scrape colors from (hyprsettings.toml, colors.conf, colors.json, etc.)
# If missing, automatically searches end-4 dots Material You and Hyprland themes.
source_file = "~/.config/hypr/hyprsettings.toml"

# Maximum accent colors passed to the model
max_accents = 4

# Fallback palette if no dotfiles or themes are found
fallback_bg = "#1e1e2e"
fallback_accents = ["#89b4fa", "#cba6f7", "#f38ba8", "#a6e3a1"]

[wallpaper]
# Backend: "auto", "quickshell", "swww", "hyprpaper", "mpvpaper", "feh", or "none"
backend = "auto"

# Save directory for generated wallpapers
save_dir = "~/Pictures/Wallpapers"

# Cache directory for temporary downloads and latest reference
cache_dir = "~/.cache/rice-gen"

# Automatically run matugen to re-theme system colors after wallpaper is set
sync_matugen = false

[wallpaper.swww]
args = ["--transition-type", "wipe", "--transition-angle", "30", "--transition-fps", "120"]

[wallpaper.mpvpaper]
args = ["-f", "--auto-pause", "-o", "no-audio loop"]

[lucky]
prompts_file = "~/.config/rice-gen/prompts.toml"
"##;

pub const DEFAULT_PROMPTS_TEMPLATE: &str = r##"# Curated prompts for `rice-gen --lucky`
# Location: ~/.config/rice-gen/prompts.toml

prompts = [
  "minimalist cyberpunk street alley at dusk, neon reflections on wet asphalt",
  "ancient torii gate in mist, glowing lanterns, cherry blossom trees, serene landscape",
  "cozy anime bedroom overlooking rainy tokyo skyline at night, warm lamplight",
  "dark fantasy towering gothic castle amidst swirling purple aurora, mysterious fog",
  "retro 80s synthwave horizon with low poly mountains and wireframe sun",
  "solarpunk botanical greenhouse floating among sky islands, lush foliage",
  "samurai standing in a field of tall autumn grass at golden hour, wind blowing",
  "deep space nebula vortex with vibrant stars and cosmic dust, high detail",
  "quiet japanese shrine in a snowy mountain forest at twilight, glowing red lanterns",
  "futuristic bullet train gliding through an infinite pastel canyon at sunrise",
  "underwater bioluminescent coral reef city, soft ambient glow, surreal atmosphere",
  "mountain monastery perched on a sheer cliff edge, fog rolling through the valley below"
]
"##;

impl Config {
    /// Load config from `~/.config/rice-gen/config.toml`, auto-initializing on first run.
    pub fn load() -> Result<Self> {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("rice-gen");
        let config_path = config_dir.join("config.toml");
        let prompts_path = config_dir.join("prompts.toml");

        // First run auto-initialization
        if !config_path.exists() {
            let _ = std::fs::create_dir_all(&config_dir);

            let sys_config = PathBuf::from("/usr/share/rice-gen/config.toml");
            if sys_config.exists() {
                let _ = std::fs::copy(&sys_config, &config_path);
            } else {
                let _ = std::fs::write(&config_path, DEFAULT_CONFIG_TEMPLATE);
            }

            if !prompts_path.exists() {
                let sys_prompts = PathBuf::from("/usr/share/rice-gen/prompts.toml");
                if sys_prompts.exists() {
                    let _ = std::fs::copy(&sys_prompts, &prompts_path);
                } else {
                    let _ = std::fs::write(&prompts_path, DEFAULT_PROMPTS_TEMPLATE);
                }
            }
        }

        if config_path.exists() {
            let text = std::fs::read_to_string(&config_path)
                .with_context(|| format!("Failed to read {}", config_path.display()))?;
            let cfg: Config = toml::from_str(&text)
                .with_context(|| format!("Failed to parse {}", config_path.display()))?;
            Ok(cfg)
        } else {
            Ok(Config {
                api: default_api(),
                palette: default_palette(),
                wallpaper: default_wallpaper(),
                lucky: LuckyConfig::default(),
            })
        }
    }

    /// Resolve the API key from `config.api.api_key` or the environment variable named in config.
    pub fn api_key(&self) -> Result<String> {
        if let Some(ref key) = self.api.api_key {
            let trimmed = key.trim();
            if !trimmed.is_empty() {
                return Ok(trimmed.to_string());
            }
        }
        std::env::var(&self.api.api_key_env).with_context(|| {
            format!(
                "${} not set and no api_key specified in ~/.config/rice-gen/config.toml",
                self.api.api_key_env
            )
        })
    }

    pub fn cache_dir(&self) -> PathBuf {
        expand_tilde(&self.wallpaper.cache_dir)
    }

    pub fn save_dir(&self) -> PathBuf {
        expand_tilde(&self.wallpaper.save_dir)
    }
}

/// Lucky prompts file schema.
#[derive(Debug, Deserialize)]
pub struct PromptsFile {
    pub prompts: Vec<String>,
}

/// Hardcoded fallback prompts (compiled into the binary).
pub const DEFAULT_PROMPTS: &[&str] = &[
    "cyberpunk noodle bar in neon rain, cinematic wide shot, detailed reflections",
    "minimalist brutalist monolith rising out of a foggy serene lake at dawn",
    "solarpunk greenhouse overgrown with lush glowing bioluminescent plants, isometric",
    "retro 90s anime spaceship bridge with CRT monitors and starfield panorama",
    "quiet japanese shrine in a snowy mountain forest at twilight, glowing lanterns",
    "futuristic bullet train gliding through an infinite pastel canyon at golden hour",
    "cozy lo-fi attic study room, cat on desk, warm lamplight, rain on the window",
    "dark fantasy ancient ruins with glowing runic obelisk, mysterious mist",
    "abandoned space station observation deck, deep space nebula visible through glass",
    "samurai standing in a field of tall grass at sunset, wind blowing, cinematic",
    "underwater bioluminescent coral reef city, soft ambient glow",
    "mountain monastery perched on a cliff edge, fog rolling through the valley below",
];

/// Load lucky prompts from file or return compiled defaults.
pub fn load_prompts(config: &Config) -> Vec<String> {
    let path = expand_tilde(&config.lucky.prompts_file);
    if path.exists() {
        if let Ok(text) = std::fs::read_to_string(&path) {
            if let Ok(pf) = toml::from_str::<PromptsFile>(&text) {
                if !pf.prompts.is_empty() {
                    return pf.prompts;
                }
            }
        }
    }
    DEFAULT_PROMPTS.iter().map(|s| s.to_string()).collect()
}

