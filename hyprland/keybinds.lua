-- rice-gen Hyprland Lua keybindings snippet
-- Drop into ~/.config/hypr/custom/keybinds.lua

-- 1. Lucky wallpaper: generate and apply in background with desktop notification
hl.bind("SUPER + SHIFT + W", hl.dsp.exec_cmd("rice-gen --lucky --notify --quiet"), { description = "Generate AI wallpaper" })

-- 2. Interactive prompt: opens floating Kitty terminal with inline preview & confirmation
hl.bind("SUPER + ALT + W", hl.dsp.exec_cmd("kitty --class rice-gen -e bash -c 'read -p \"Enter Wallpaper Prompt: \" p && rice-gen \"$p\" -p'"), { description = "Prompt AI wallpaper" })

-- 3. Save active wallpaper to collection
hl.bind("SUPER + CTRL + W", hl.dsp.exec_cmd("rice-gen save"), { description = "Save active wallpaper" })

