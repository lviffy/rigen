use std::io::{self, IsTerminal, Write};
use std::path::Path;
use std::process::Command;

use anyhow::Result;
use colored::Colorize;

use crate::palette::ExtractedPalette;

#[derive(Debug, PartialEq, Eq)]
pub enum PreviewAction {
    Apply,
    Discard,
    Regenerate,
    Save,
}

/// Detect if running in Kitty terminal emulator.
pub fn is_kitty() -> bool {
    std::env::var("TERM").map(|t| t.contains("kitty")).unwrap_or(false)
        || std::env::var("KITTY_PID").is_ok()
}

/// Render the image preview in the terminal and prompt the user for action.
pub fn show_preview(
    image_path: &Path,
    prompt: &str,
    palette: &ExtractedPalette,
    provider: &str,
    duration_secs: f64,
) -> Result<PreviewAction> {
    // If not attached to a terminal, proceed automatically
    if !io::stdin().is_terminal() {
        return Ok(PreviewAction::Apply);
    }

    println!();
    println!("{}", "── Generated Wallpaper Preview ──".dimmed());

    // Try rendering using viuer (supports Kitty graphics protocol, Sixel, and block characters)
    let viuer_conf = viuer::Config {
        transparent: true,
        absolute_offset: false,
        restore_cursor: false,
        width: Some(60),
        ..Default::default()
    };

    let rendered = viuer::print_from_file(image_path, &viuer_conf);

    if rendered.is_err() && is_kitty() {
        // Fallback to kitty icat kitten
        let _ = Command::new("kitty")
            .args(["+kitten", "icat", "--align", "left"])
            .arg(image_path)
            .status();
    }

    // Print palette swatches
    if palette.is_free {
        println!("\n  {} {}", "Palette:".bold(), "Natural (Inferred from Prompt)".green().italic());
    } else {
        print!("\n  {} ", "Palette:".bold());
        print!(
            "{} ",
            format!("██ {}", palette.background.to_hex()).truecolor(
                palette.background.r,
                palette.background.g,
                palette.background.b
            )
        );
        for accent in &palette.accents {
            print!(
                "{} ",
                format!("██ {}", accent.to_hex()).truecolor(accent.r, accent.g, accent.b)
            );
        }
        println!();
    }

    if let Some(ref theme) = palette.theme_name {
        println!("  {}     {}", "Theme:".bold(), theme.cyan());
    }
    println!("  {}    \"{}\"", "Prompt:".bold(), prompt.italic());
    println!(
        "  {}  {} ({:.2}s)",
        "Engine:".bold(),
        provider.yellow(),
        duration_secs
    );

    println!();
    loop {
        print!(
            "  Apply? [{}/n/r/s] ({}: Apply, {}: Discard, {}: Regenerate, {}: Save): ",
            "Y".bold().green(),
            "Y".green(),
            "n".red(),
            "r".yellow(),
            "s".blue()
        );
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let choice = input.trim().to_lowercase();

        match choice.as_str() {
            "" | "y" | "yes" => return Ok(PreviewAction::Apply),
            "n" | "no" | "q" | "quit" => return Ok(PreviewAction::Discard),
            "r" | "regen" | "regenerate" => return Ok(PreviewAction::Regenerate),
            "s" | "save" => return Ok(PreviewAction::Save),
            other => {
                println!("  Unknown option '{}'. Enter 'y', 'n', 'r', or 's'.", other);
            }
        }
    }
}

