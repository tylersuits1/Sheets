# Sheets

A desktop app for managing terminal themes, fonts, and opacity across
Ghostty, Kitty, and Alacritty — with color-wheel pickers, live preview, and
theme export for sharing your own creations.

## Status

Early scaffolding — not yet functional.

## Planned features

- Color wheel / HEX input for foreground, background, and palette colors
- Opacity slider
- Font family + size picker
- Live preview pane (sample code block + pixelated splash icon recoloring)
- Create custom themes by hand and export them as `sheets-theme.json`,
  tracked as "user installed" vs. pre-installed
- Tag themes as Dark / Light / Both
- Cross-platform: macOS and Linux (Omarchy), auto-detecting each app's
  config file location
- Supports Ghostty, Kitty, and Alacritty (Alacritty via a TOML adapter,
  the other two via flat key-value config parsing)
- Distributed via tylersuits.com/sheets and GitHub Releases

## Theme repo format

File > Export Theme writes a user-created theme as `sheets-theme.json`, meant
to sit at the root of a git repo (or wherever you'd like to share it) so
others can pick it up:

```json
{
  "name": "Tokyo Night",
  "variant": "dark",
  "palette": {
    "background": "1a1b26",
    "foreground": "c0caf5",
    "cursor": "c0caf5",
    "selection_background": "283457",
    "selection_foreground": null,
    "ansi": [
      "15161e", "f7768e", "9ece6a", "e0af68",
      "7aa2f7", "bb9af7", "7dcfff", "a9b1d6",
      "414868", "f7768e", "9ece6a", "e0af68",
      "7aa2f7", "bb9af7", "7dcfff", "c0caf5"
    ]
  }
}
```

- `variant` is `"dark"`, `"light"`, or `"both"`.
- Colors are hex strings without a leading `#`.
- `ansi` is exactly 16 entries: the standard ANSI 0-15 order (black, red,
  green, yellow, blue, magenta, cyan, white, then the bright variants of
  each).
- `cursor`, `selection_background`, and `selection_foreground` are
  optional.

This is Sheets' own format, not a wrapper around an existing standard
(base16, iTerm color schemes, etc.).

## Stack

- [Tauri](https://tauri.app/) (Rust backend + web frontend, vanilla TS)
  for small, native cross-platform builds
- Reference app: [Ghostty](https://ghostty.org/) ([docs](https://ghostty.org/docs))

## Dev setup

```bash
npm install
npm run tauri dev
```

Recommended IDE setup: [VS Code](https://code.visualstudio.com/) +
[Tauri extension](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) +
[rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
