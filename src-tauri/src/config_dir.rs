use std::path::PathBuf;

/// The XDG-style config directory used by Ghostty, Kitty, and Alacritty on
/// both macOS and Linux (including Omarchy): `$XDG_CONFIG_HOME`, falling
/// back to `~/.config`. All three apps honor this on every platform they
/// support, so no per-OS branching is needed here.
pub fn xdg_config_home() -> Result<PathBuf, String> {
    if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
        if !dir.is_empty() {
            return Ok(PathBuf::from(dir));
        }
    }
    let home = std::env::var("HOME")
        .map_err(|_| "HOME environment variable is not set".to_string())?;
    Ok(PathBuf::from(home).join(".config"))
}

/// Sheets' own data directory, for the user-theme manifest.
pub fn sheets_data_dir() -> Result<PathBuf, String> {
    Ok(xdg_config_home()?.join("sheets"))
}

/// Ghostty on macOS loads `~/.config/ghostty/config` (XDG) AND a
/// macOS-native file under `~/Library/Application Support/com.mitchellh.ghostty/`,
/// with the macOS one loaded *after* — so it silently wins any conflict,
/// confirmed against a real machine where a theme applied to the XDG file
/// never took visible effect because of stray values sitting in this file.
/// Ghostty accepts either `config` or `config.ghostty` there; prefer
/// whichever already exists so Sheets edits the file actually in play,
/// defaulting to `config.ghostty` (Ghostty's macOS-specific name) if
/// neither exists yet.
pub fn ghostty_macos_config_path() -> Result<PathBuf, String> {
    let home = std::env::var("HOME").map_err(|_| "HOME environment variable is not set".to_string())?;
    let dir = PathBuf::from(home).join("Library/Application Support/com.mitchellh.ghostty");
    let dotted = dir.join("config.ghostty");
    let plain = dir.join("config");
    if plain.exists() && !dotted.exists() {
        Ok(plain)
    } else {
        Ok(dotted)
    }
}
