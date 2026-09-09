# Sheets

A desktop app for managing terminal themes, fonts, and transparency across
Ghostty, Kitty, and Alacritty — with color-wheel pickers, live preview, and
one-click theme install from git repos.

## Status

Early scaffolding — not yet functional.

## Planned features

- Color wheel / HEX input for foreground, background, and palette colors
- Transparency slider
- Font family + size picker
- Live preview pane (sample code block + pixelated splash icon recoloring)
- Install new themes from a git URL, tracked as "user installed" vs.
  pre-installed
- Tag themes as Dark / Light / Both
- Cross-platform: macOS and Linux (Omarchy), auto-detecting each app's
  config file location
- Supports Ghostty, Kitty, and Alacritty (Alacritty via a TOML adapter,
  the other two via flat key-value config parsing)
- Distributed via tylersuits.com/sheets and GitHub Releases

## Stack

- [Tauri](https://tauri.app/) (Rust backend + web frontend) for small,
  native cross-platform builds
- Reference app: [Ghostty](https://ghostty.org/) ([docs](https://ghostty.org/docs))
