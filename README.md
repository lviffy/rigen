# `rice-gen`
**AI-Powered Palette-Synchronized Wallpaper Generator for Hyprland & Linux**

Synthesize desktop wallpapers that dynamically match your exact active rice color scheme using **Recraft V4.1 Flash** (via OpenRouter or native Recraft API).

---

## Features

- **Auto-Extracts Theme Colors:** Directly scrapes your active theme (`Catppuccin Mocha`, etc.) from `~/.config/hypr/hyprsettings.toml` or extracts Hyprlang `rgba()` / `rgb()` values.
- **Native RGB Color Anchoring:** Injects your background and top accent colors as strict RGB constraints (`image_config.rgb_colors` on OpenRouter, `controls.colors` on Recraft).
- **Sub-5s Synthesis:** Generates 16:9 widescreen wallpapers using the lightning-fast Recraft V4.1 Flash model.
- **Free Tier Ready:** Supports OpenRouter's free tier model `recraft/recraft-v4.1-flash-v2:free`.
- **Automatic Monitor Upscaling:** Automatically scales downloaded images to your physical screen resolution (e.g., 1920x1080) using high-quality Lanczos3 filtering, ensuring razor-sharp wallpapers without aspect distortion or upscale prompts.
- **Auto-Saves to Pictures:** Automatically archives each generated wallpaper with a timestamp (`rice-gen_YYYYMMDD_HHMMSS.png`) and JSON metadata sidecar directly into `~/Pictures/Wallpapers/`.
- **Full end-4 dots (Illogical Impulse) Compatibility:** Out-of-the-box support for all versions of end-4's `dots-hyprland`. Reads dynamic Material You colors (`colors.json`), respects `$qsConfig`, and triggers QuickShell IPC or `switchwall.sh` seamlessly without breaking dotfiles.
- **Quickshell & Desktop Dispatch:** Natively interfaces with QuickShell (`ii`, `illogical-impulse`, `lviffy-shell`) shader transition animations via IPC, with automatic fallbacks to `swww`, `hyprpaper`, `mpvpaper`, and `feh`.
- **Interactive Kitty Terminal Preview:** Inline graphics rendering via Kitty Graphics Protocol (`viuer` or `icat`) with an interactive `[Y/n/r/s]` menu (Apply, Discard, Regenerate, Save).
- **Desktop Notifications:** Desktop alerts with image thumbnails sent via `notify-send`.

---

## Prerequisites

Make sure the following dependencies are installed on your system:

### 1. Build Tools (Rust & Cargo)
```bash
# Arch Linux
sudo pacman -S rust cargo git

# Or via rustup (any distro)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 2. Recommended Runtime Tools
- **Hyprland** (Wayland compositor)
- **Kitty** (Optional: for inline terminal previews)
- **libnotify** (`notify-send` for desktop notifications)
- **Wallpaper Backend** (Quickshell `lviffy-shell`, `swww`, `hyprpaper`, `mpvpaper`, or `feh`)

---

## Download & Installation

### Option 1: Arch Linux (Manual `makepkg` / AUR Build) — Recommended

`rice-gen` includes a fully compliant, self-contained Arch Linux `PKGBUILD`. Anyone on Arch Linux (or any Arch-based distribution) can build and install it out-of-the-box without waiting for remote AUR hosting:

```bash
# 1. Clone the repository
git clone https://github.com/lviffy/rice-gen.git
cd rice-gen

# 2. Build and install package + optional dependencies
makepkg -si
```

#### What `makepkg -si` sets up automatically:
- Installs the compiled release binary to `/usr/bin/rice-gen`.
- Installs default configuration templates to `/usr/share/rice-gen/` (auto-initialized to `~/.config/rice-gen/` on your first run).
- Installs Hyprland configuration snippets (both standard Hyprlang and Lua) to `/usr/share/rice-gen/hyprland/`.
- Tracks the installation in `pacman` for clean upgrades and removals (`sudo pacman -R rice-gen`).

---

### Option 2: Standalone Cargo Build (Any Linux Distro)

If you prefer building directly with Cargo or are on Fedora, Debian, Ubuntu, or openSUSE:

```bash
# 1. Clone the repository
git clone https://github.com/lviffy/rice-gen.git ~/.config/hypr/rigen
cd ~/.config/hypr/rigen

# 2. Build the optimized release binary
cargo build --release

# 3. Copy executable to your user PATH
mkdir -p ~/.local/bin
cp target/release/rice-gen ~/.local/bin/
chmod +x ~/.local/bin/rice-gen
```

Make sure `~/.local/bin` is in your `$PATH`. In `~/.bashrc`, `~/.zshrc`, or `~/.config/fish/config.fish`:
```bash
export PATH="$HOME/.local/bin:$PATH"
```

Verify your installation:
```bash
rice-gen --version
```

---

## Configuration

> **Note:** On first launch, `rice-gen` automatically detects if `~/.config/rice-gen/` exists. If not, it self-initializes with complete, ready-to-use configuration and prompt files!

### 1. Configuration File (`~/.config/rice-gen/config.toml`)
If you wish to edit your configuration manually:

```bash
# Generated automatically on first run, or create manually:
mkdir -p ~/.config/rice-gen
```

Edit `~/.config/rice-gen/config.toml`:

```toml
[api]
# Provider: "openrouter" or "recraft"
provider = "openrouter"

# OpenRouter Model:
# Free tier: "recraft/recraft-v4.1-flash-v2:free"
# Paid tier: "recraft/recraft-v4.1-flash-v2"
model = "recraft/recraft-v4.1-flash-v2:free"

# Environment variable name holding your API key
api_key_env = "OPENROUTER_API_KEY"

# Optional: Direct API key fallback if not set in environment
api_key = "sk-or-v1-..."

# Default aspect ratio ("16:9", "16:10", "21:9", "4:3", "1:1")
aspect_ratio = "16:9"

[palette]
# Dotfile to scrape colors from (hyprsettings.toml, colors.conf, etc.)
source_file = "~/.config/hypr/hyprsettings.toml"

# Maximum accent colors passed to the model
max_accents = 4

# Fallback palette if source file is missing
fallback_bg = "#1e1e2e"
fallback_accents = ["#89b4fa", "#cba6f7", "#f38ba8", "#a6e3a1"]

[wallpaper]
# Backend: "auto", "quickshell", "swww", "hyprpaper", "mpvpaper", "feh", or "none"
backend = "auto"

# Directories for wallpapers and temporary cache
save_dir = "~/Pictures/Wallpapers"
cache_dir = "~/.cache/rice-gen"

# Automatically run matugen after setting wallpaper
sync_matugen = false

[lucky]
# Curated prompt list for --lucky random generations
prompts_file = "~/.config/rice-gen/prompts.toml"
```

### 2. Set Up API Key
Obtain a free API key from [OpenRouter](https://openrouter.ai/settings/keys) and add it to your environment:

In `~/.bashrc` or `~/.zshrc`:
```bash
export OPENROUTER_API_KEY="sk-or-v1-your-key-here"
```

For Hyprland environments:
- **`~/.config/hypr/custom/env.conf`**:
  ```ini
  env = OPENROUTER_API_KEY,sk-or-v1-your-key-here
  ```
- **`~/.config/hypr/custom/env.lua`**:
  ```lua
  hl.setenv("OPENROUTER_API_KEY", "sk-or-v1-your-key-here")
  ```

### 3. Lucky Prompts (`~/.config/rice-gen/prompts.toml`)
Create `~/.config/rice-gen/prompts.toml` with curated aesthetic prompts:

```toml
prompts = [
  "minimalist cyberpunk street alley at dusk, neon reflections on wet asphalt",
  "ancient torii gate in mist, glowing lanterns, cherry blossom trees",
  "cozy anime bedroom overlooking rainy tokyo skyline at night",
  "dark fantasy towering gothic castle amidst swirling purple aurora",
  "retro 80s synthwave horizon with low poly mountains and wireframe sun",
  "solarpunk botanical greenhouse floating among sky islands",
  "samurai standing in a field of tall autumn grass at golden hour",
  "deep space nebula vortex with vibrant stars and cosmic dust"
]
```

---

## Usage

### 1. Generate with Prompt (Natural Prompt Thinking)
By default, the AI model thinks freely from your prompt rather than being clamped to a single default desktop palette:
```bash
# Model determines the natural colors of the scene:
rice-gen "misty bamboo forest in morning rain"

# Provide colors directly in your prompt (named or hex):
rice-gen "cyberpunk motorcycle in neon pink and cyan"
rice-gen "samurai in crimson red and gold armor"
rice-gen "retro grid horizon in #1a0826, #ff71ce, #01cdfe"
```

### 2. Lucky Roll (Random Prompt & Random Aesthetic Palette)
Picks a random curated prompt and pairs it with a vibrant **random aesthetic palette** (Cyberpunk, Synthwave, Tokyo Midnight, Sakura Bloom, Solar Flare, Emerald Rainforest, etc.):
```bash
rice-gen --lucky --notify
```
*(Every lucky wallpaper has a fresh, unique aesthetic color palette!)*

### 3. Synchronize with Active Desktop Rice Theme (`--rice`)
Explicitly lock the wallpaper to your active Hyprland desktop theme (`Catppuccin Mocha`, `hyprsettings.toml`, or Material You `colors.json`):
```bash
rice-gen "anime girl under rainy street lamp" --rice
```

### 4. Interactive Terminal Preview (`-p`)
Renders the generated wallpaper inside your Kitty terminal with a confirmation prompt:
```bash
rice-gen "solarpunk botanical observatory" --preview
```
*Keys: `y` (Apply), `n` (Discard), `r` (Regenerate), `s` (Save).*

### 5. Custom Palette Override (`-c` / `--random-palette`)
```bash
# Pass custom hex accents:
rice-gen "deep sea abyss" --colors "#0f172a,#38bdf8,#818cf8"

# Force a random aesthetic palette on any custom prompt:
rice-gen "cybernetic samurai" --random-palette
```

### 6. Inspect Active Palette & Settings
Displays your current active desktop rice palette and configuration:
```bash
rice-gen current
```

### 7. View Wallpaper History
Lists recently generated wallpapers, timestamps, and extracted palettes:
```bash
rice-gen history
```

---

## Hyprland Integration

### 1. Instant Drop-in (Installed via Arch PKGBUILD)

If you installed `rice-gen` via `makepkg -si`, the configuration snippets are already located at `/usr/share/rice-gen/hyprland/`. You can source them directly into your Hyprland configuration:

**Standard Hyprlang (`~/.config/hypr/hyprland.conf` or `custom/`):**
```ini
source = /usr/share/rice-gen/hyprland/keybinds.conf
source = /usr/share/rice-gen/hyprland/rules.conf
```

**end-4 / Lua Hyprland (`~/.config/hypr/hyprland.lua` or `custom/`):**
```lua
dofile("/usr/share/rice-gen/hyprland/keybinds.lua")
dofile("/usr/share/rice-gen/hyprland/rules.lua")
```

---

### 2. Manual Keybindings & Rules

Alternatively, you can copy the keybindings and window rules directly into your existing configuration files:

#### Keybindings
Add to `~/.config/hypr/custom/keybinds.conf`:
```ini
# Generate lucky wallpaper in background with notification
bindd = Super+Shift, W, Generate AI wallpaper, exec, rice-gen --lucky --notify --quiet

# Open floating prompt dialog with terminal preview
bindd = Super+Alt, W, Prompt AI wallpaper, exec, kitty --class rice-gen -e bash -c 'read -p "Enter Wallpaper Prompt: " p && rice-gen "$p" -p'

# Save current wallpaper to collection
bindd = Super+Ctrl, W, Save active wallpaper, exec, rice-gen save
```

If using Hyprland with Lua (`~/.config/hypr/custom/keybinds.lua`):
```lua
hl.bind("SUPER + SHIFT + W", hl.dsp.exec_cmd("rice-gen --lucky --notify --quiet"), { description = "Generate AI wallpaper" })
hl.bind("SUPER + ALT + W", hl.dsp.exec_cmd("kitty --class rice-gen -e bash -c 'read -p \"Enter Wallpaper Prompt: \" p && rice-gen \"$p\" -p'"), { description = "Prompt AI wallpaper" })
hl.bind("SUPER + CTRL + W", hl.dsp.exec_cmd("rice-gen save"), { description = "Save active wallpaper" })
```

### Floating Window Rules

Make the interactive prompt window float nicely in the center of the screen:

Add to `~/.config/hypr/custom/rules.conf`:
```ini
windowrulev2 = float, class:^(rice-gen)$
windowrulev2 = size 950 650, class:^(rice-gen)$
windowrulev2 = center, class:^(rice-gen)$
```

Add to `~/.config/hypr/custom/rules.lua`:
```lua
hl.windowrule("float", { class = "^(rice-gen)$" })
hl.windowrule("size", "950 650", { class = "^(rice-gen)$" })
hl.windowrule("center", "", { class = "^(rice-gen)$" })
```

Reload Hyprland:
```bash
hyprctl reload
```

---

## License

MIT License. Crafted with precision for the Linux ricing community.
