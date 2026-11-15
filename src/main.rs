mod api;
mod config;
mod notify;
mod palette;
mod preview;
mod save;
mod setter;

use std::path::PathBuf;
use std::time::Instant;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use rand::seq::IndexedRandom;

use crate::config::Config;
use crate::preview::PreviewAction;
use crate::setter::{resolve_backend, run_post_hooks, set_wallpaper};

#[derive(Parser, Debug)]
#[command(
    name = "rice-gen",
    author = "lviffy",
    version = "0.1.0",
    about = "AI-powered palette-synchronized wallpaper generator for Hyprland"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Text prompt describing the desired wallpaper (optional if --lucky)
    #[arg(value_name = "PROMPT")]
    pub prompt: Option<String>,

    /// Override palette with comma-separated hex codes (e.g. "#1e1e2e,#cba6f7")
    #[arg(short = 'c', long = "colors", value_delimiter = ',')]
    pub colors: Option<Vec<String>>,

    /// Path to dotfile to scrape hex colors from
    #[arg(short = 's', long = "source")]
    pub source: Option<String>,

    /// Wallpaper setter backend (auto, quickshell, mpvpaper, swww, hyprpaper, feh, none)
    #[arg(short = 'b', long = "backend")]
    pub backend: Option<String>,

    /// Aspect ratio (16:9, 16:10, 21:9, 4:3, 1:1)
    #[arg(short = 'a', long = "aspect")]
    pub aspect: Option<String>,

    /// API provider override (openrouter, recraft)
    #[arg(long = "provider")]
    pub provider: Option<String>,

    /// Render image in terminal (Kitty Graphics Protocol) before applying
    #[arg(short = 'p', long = "preview")]
    pub preview: bool,

    /// Pick a random prompt from prompts.toml
    #[arg(short = 'l', long = "lucky")]
    pub lucky: bool,

    /// Send desktop notification via notify-send with thumbnail
    #[arg(short = 'n', long = "notify")]
    pub notify: bool,

    /// Run `matugen image <path>` after setting wallpaper
    #[arg(long = "sync-matugen")]
    pub sync_matugen: bool,

    /// Custom output path (defaults to auto-generated path in save_dir)
    #[arg(short = 'o', long = "output")]
    pub output: Option<PathBuf>,

    /// Explicitly synchronize with active Hyprland desktop rice theme colors
    #[arg(long = "rice")]
    pub rice: bool,

    /// Force a completely random aesthetic color palette
    #[arg(long = "random-palette")]
    pub random_palette: bool,

    /// Suppress all stdout output (useful for keybindings/scripts)
    #[arg(short = 'q', long = "quiet")]
    pub quiet: bool,

    /// Enable verbose debug output
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Save the most recently generated wallpaper to permanent storage
    Save {
        /// Optional name label for the saved wallpaper
        name: Option<String>,
    },
    /// List recently generated/saved wallpapers
    History,
    /// Print active extracted palette and configuration details
    Current,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut config = Config::load()?;

    // Apply CLI overrides to config
    if let Some(backend) = cli.backend {
        config.wallpaper.backend = backend;
    }
    if let Some(aspect) = cli.aspect {
        config.api.aspect_ratio = aspect;
    }
    if let Some(provider) = cli.provider {
        config.api.provider = provider;
    }

    // Handle Subcommands
    match cli.command {
        Some(Commands::Save { name }) => {
            let saved_path = save::save_current(&config, name)?;
            if !cli.quiet {
                println!(
                    "{} Wallpaper saved to {}",
                    "✔".green().bold(),
                    saved_path.display().to_string().cyan()
                );
            }
            return Ok(());
        }
        Some(Commands::History) => {
            save::list_history(&config)?;
            return Ok(());
        }
        Some(Commands::Current) => {
            let palette = palette::resolve_desktop_palette(&config)?;
            println!("\n  ── Active Desktop Rice Palette ──");
            if let Some(ref theme) = palette.theme_name {
                println!("  Theme:        {}", theme.cyan().bold());
            }
            println!(
                "  Background:   {} ({})",
                format!("██ {}", palette.background.to_hex()).truecolor(
                    palette.background.r,
                    palette.background.g,
                    palette.background.b
                ),
                palette.background.to_hex()
            );
            print!("  Accents:      ");
            for accent in &palette.accents {
                print!(
                    "{} ",
                    format!("██ {}", accent.to_hex()).truecolor(accent.r, accent.g, accent.b)
                );
            }
            println!("\n");
            println!("  Palette Mode: {}", config.palette.mode.cyan());
            println!("  Provider:     {}", config.api.provider.yellow());
            println!("  Model:        {}", config.api.model);
            println!("  Aspect:       {}", config.api.aspect_ratio);
            let backend = resolve_backend(&config).await;
            println!("  Backend:      {:?}", backend);
            println!("  Save Dir:     {}", config.save_dir().display());
            println!();
            return Ok(());
        }
        None => {}
    }

    // Resolve Prompt
    let prompt = if cli.lucky {
        let prompts = config::load_prompts(&config);
        let mut rng = rand::rng();
        prompts
            .choose(&mut rng)
            .cloned()
            .unwrap_or_else(|| "minimalist dark aesthetic landscape".to_string())
    } else if let Some(p) = cli.prompt {
        p
    } else {
        bail!("No prompt provided. Pass a prompt string or use --lucky to pick randomly.");
    };

    // Resolve Palette
    let palette = palette::resolve_palette(
        &cli.colors,
        &cli.source,
        &prompt,
        cli.lucky,
        cli.rice,
        cli.random_palette,
        &config,
    )?;

    if !cli.quiet {
        println!("{}", "── Synthesizing Wallpaper ──".bold());
        if let Some(ref theme) = palette.theme_name {
            println!("  Theme:    {}", theme.cyan());
        }
        if palette.is_free {
            println!("  Palette:  {}", "Natural (Model Thinking from Prompt)".green().italic());
        } else {
            print!("  Palette:  ");
            print!(
                "{} ",
                format!("██ {}", palette.background.to_hex()).truecolor(
                    palette.background.r,
                    palette.background.g,
                    palette.background.b
                )
            );
            for a in &palette.accents {
                print!(
                    "{} ",
                    format!("██ {}", a.to_hex()).truecolor(a.r, a.g, a.b)
                );
            }
            println!();
        }
        println!("  Prompt:   \"{}\"", prompt.italic());
        println!("  Provider: {} ({})", config.api.provider.yellow(), config.api.model);
    }

    // Directories
    let cache_dir = config.cache_dir();
    let save_dir = config.save_dir();
    std::fs::create_dir_all(&cache_dir)?;
    std::fs::create_dir_all(&save_dir)?;

    let raw_download = cache_dir.join("raw_download.webp");

    // Unique destination path so QuickShell and image caches detect every new wallpaper
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let output_path = cli.output.unwrap_or_else(|| {
        save_dir.join(format!("rice-gen_{}.png", timestamp))
    });

    // Query screen resolution (at least 1920x1080) for upscale
    let (max_w, max_h) = setter::query_max_resolution().await;

    // Generation loop (supports 'regenerate' action in preview)
    let mut current_prompt = prompt;
    loop {
        let start_time = Instant::now();
        if !cli.quiet {
            print!("  Synthesizing with Recraft V4.1 Flash... ");
            use std::io::Write;
            let _ = std::io::stdout().flush();
        }

        api::generate(&config, &current_prompt, &palette, &raw_download).await?;
        let elapsed = start_time.elapsed().as_secs_f64();

        // Scale / upscale image to monitor resolution and save as PNG
        setter::postprocess_wallpaper(&raw_download, &output_path, max_w, max_h)?;

        // Maintain latest.png and latest.webp in cache for quick access / history
        let _ = std::fs::copy(&output_path, cache_dir.join("latest.png"));
        let _ = std::fs::copy(&raw_download, cache_dir.join("latest.webp"));

        if !cli.quiet {
            println!("{} ({:.2}s)", "Done!".green().bold(), elapsed);
        }

        // If palette was free, sample actual dominant colors from the generated image for metadata & preview
        let mut display_palette = palette.clone();
        if display_palette.is_free {
            if let Some(sampled) = palette::sample_image_palette(&output_path, config.palette.max_accents) {
                display_palette = sampled;
            }
        }

        // Record metadata sidecar in cache and beside output
        let _ = save::record_generation_meta(
            &cache_dir.join("latest.json"),
            &output_path,
            &current_prompt,
            &display_palette,
            &config,
        );
        let meta_sidecar = output_path.with_extension("json");
        let _ = save::record_generation_meta(
            &meta_sidecar,
            &output_path,
            &current_prompt,
            &display_palette,
            &config,
        );

        // Preview flow
        if cli.preview {
            let action = preview::show_preview(
                &output_path,
                &current_prompt,
                &display_palette,
                &config.api.provider,
                elapsed,
            )?;

            match action {
                PreviewAction::Apply => break,
                PreviewAction::Discard => {
                    if !cli.quiet {
                        println!("  Wallpaper discarded.");
                    }
                    return Ok(());
                }
                PreviewAction::Regenerate => {
                    if cli.lucky {
                        let prompts: Vec<String> = config::load_prompts(&config);
                        let mut rng = rand::rng();
                        if let Some(p) = prompts.choose(&mut rng) {
                            current_prompt = p.clone();
                        }
                    }
                    if !cli.quiet {
                        println!("  Regenerating with: \"{}\"...", current_prompt.italic());
                    }
                    continue;
                }
                PreviewAction::Save => {
                    let saved = save::save_current(&config, None)?;
                    if !cli.quiet {
                        println!(
                            "{} Saved to {}",
                            "✔".green().bold(),
                            saved.display().to_string().cyan()
                        );
                    }
                    break;
                }
            }
        } else {
            break;
        }
    }

    // Set Wallpaper
    let backend = resolve_backend(&config).await;
    if !cli.quiet && backend != setter::Backend::None {
        println!("  Applying wallpaper via {:?}...", backend);
    }

    set_wallpaper(&config, &backend, &output_path).await?;

    // Post-Hooks (matugen, custom script, notification)
    run_post_hooks(
        &config,
        &output_path,
        &current_prompt,
        cli.sync_matugen,
        cli.notify,
    )
    .await?;

    if !cli.quiet {
        println!(
            "{} Desktop updated successfully! ({})",
            "✔".green().bold(),
            output_path.display().to_string().cyan()
        );
    }

    Ok(())
}
