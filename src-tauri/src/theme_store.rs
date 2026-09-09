use crate::config_dir::sheets_data_dir;
use crate::theme::{Palette, Period, Theme, ThemeSource, ThemeVariant};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn manifest_path() -> Result<PathBuf, String> {
    Ok(sheets_data_dir()?.join("themes.json"))
}

fn built_in_themes() -> Vec<Theme> {
    vec![
        Theme {
            id: "tokyo-night".into(),
            name: "Tokyo Night".into(),
            variant: ThemeVariant::Dark,
            source: ThemeSource::BuiltIn,
            git_url: None,
            palette: Palette {
                background: "1a1b26".into(),
                foreground: "c0caf5".into(),
                cursor: Some("c0caf5".into()),
                selection_background: Some("283457".into()),
                selection_foreground: None,
                ansi: [
                    "15161e", "f7768e", "9ece6a", "e0af68", "7aa2f7", "bb9af7", "7dcfff", "a9b1d6",
                    "414868", "f7768e", "9ece6a", "e0af68", "7aa2f7", "bb9af7", "7dcfff", "c0caf5",
                ]
                .map(String::from),
            },
        },
        Theme {
            id: "solarized-light".into(),
            name: "Solarized Light".into(),
            variant: ThemeVariant::Light,
            source: ThemeSource::BuiltIn,
            git_url: None,
            palette: Palette {
                background: "fdf6e3".into(),
                foreground: "657b83".into(),
                cursor: Some("657b83".into()),
                selection_background: Some("eee8d5".into()),
                selection_foreground: None,
                ansi: [
                    "073642", "dc322f", "859900", "b58900", "268bd2", "d33682", "2aa198", "eee8d5",
                    "002b36", "cb4b16", "586e75", "657b83", "839496", "6c71c4", "93a1a1", "fdf6e3",
                ]
                .map(String::from),
            },
        },
    ]
}

fn load_user_themes() -> Result<Vec<Theme>, String> {
    let path = manifest_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let contents = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&contents).map_err(|e| e.to_string())
}

fn save_user_themes(themes: &[Theme]) -> Result<(), String> {
    let path = manifest_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let contents = serde_json::to_string_pretty(themes).map_err(|e| e.to_string())?;
    fs::write(path, contents).map_err(|e| e.to_string())
}

pub fn list_themes() -> Result<Vec<Theme>, String> {
    let mut themes = built_in_themes();
    themes.extend(crate::ghostty_import::list_bundled_themes());
    themes.extend(load_user_themes()?);
    Ok(themes)
}

pub fn get_theme(id: &str) -> Result<Theme, String> {
    list_themes()?
        .into_iter()
        .find(|t| t.id == id)
        .ok_or_else(|| format!("no theme with id \"{id}\""))
}

pub fn list_themes_for_period(period: Period) -> Result<Vec<Theme>, String> {
    Ok(list_themes()?.into_iter().filter(|t| period.accepts(t.variant)).collect())
}

fn palettes_match(a: &Palette, b: &Palette) -> bool {
    a.background.eq_ignore_ascii_case(&b.background)
        && a.foreground.eq_ignore_ascii_case(&b.foreground)
        && a.ansi.iter().zip(b.ansi.iter()).all(|(x, y)| x.eq_ignore_ascii_case(y))
}

/// Reverse-matches a palette read from a live config file back to a known
/// theme (built-in or user-installed). `Ok(None)` means the colors don't
/// match anything Sheets knows about — a custom/hand-edited config.
pub fn identify_theme(palette: &Palette) -> Result<Option<Theme>, String> {
    Ok(list_themes()?.into_iter().find(|t| palettes_match(&t.palette, palette)))
}

pub fn remove_user_theme(id: &str) -> Result<(), String> {
    let mut themes = load_user_themes()?;
    let before = themes.len();
    themes.retain(|t| t.id != id);
    if themes.len() == before {
        return Err(format!("no user-installed theme with id \"{id}\""));
    }
    save_user_themes(&themes)
}

/// Creates a theme from hand-picked colors (the "File > Create Theme"
/// window), storing it exactly like a git-installed one but with no
/// `git_url` — both are `UserInstalled`, which is what makes a theme
/// eligible for export.
pub fn save_user_theme(name: String, variant: ThemeVariant, palette: Palette) -> Result<Theme, String> {
    if name.trim().is_empty() {
        return Err("theme name can't be empty".to_string());
    }
    let theme = Theme { id: slugify(&name), name, variant, source: ThemeSource::UserInstalled, git_url: None, palette };

    let mut themes = load_user_themes()?;
    themes.retain(|t| t.id != theme.id);
    themes.push(theme.clone());
    save_user_themes(&themes)?;
    Ok(theme)
}

/// Serializes a user-created/installed theme back into the `sheets-theme.json`
/// shape, so it can be saved to a file and pushed to a git repo for others to
/// install via `install_theme_from_git`. Built-in themes (including ones
/// imported from Ghostty) aren't exportable — there's nothing of the user's
/// to hand off.
pub fn export_theme_json(id: &str) -> Result<String, String> {
    let theme = get_theme(id)?;
    if theme.source != ThemeSource::UserInstalled {
        return Err(format!("\"{}\" isn't a user-created theme, so there's nothing to export", theme.name));
    }
    let manifest = ThemeManifest { name: theme.name, variant: theme.variant, palette: theme.palette };
    serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())
}

/// The `sheets-theme.json` shape expected at the root of a theme repo, as
/// documented in the README's "Theme repo format" section — keep the two in
/// sync (`theme_manifest_matches_documented_format` checks the parsing side).
#[derive(serde::Serialize, serde::Deserialize)]
struct ThemeManifest {
    name: String,
    variant: ThemeVariant,
    palette: Palette,
}

fn parse_theme_manifest(contents: &str, git_url: &str) -> Result<Theme, String> {
    let parsed: ThemeManifest = serde_json::from_str(contents).map_err(|e| e.to_string())?;
    Ok(Theme {
        id: slugify(&parsed.name),
        name: parsed.name,
        variant: parsed.variant,
        source: ThemeSource::UserInstalled,
        git_url: Some(git_url.to_string()),
        palette: parsed.palette,
    })
}

/// Clones `git_url` into a scratch directory and reads a `sheets-theme.json`
/// file from its root. Repos in other formats (base16, iTerm color schemes,
/// etc.) aren't supported — `sheets-theme.json` is Sheets' own format, not a
/// wrapper around an existing standard.
pub fn install_theme_from_git(git_url: &str) -> Result<Theme, String> {
    let scratch = std::env::temp_dir().join(format!("sheets-theme-install-{}", std::process::id()));
    if scratch.exists() {
        fs::remove_dir_all(&scratch).map_err(|e| e.to_string())?;
    }

    let status = Command::new("git")
        .args(["clone", "--depth", "1", git_url, &scratch.to_string_lossy()])
        .status()
        .map_err(|e| format!("failed to run git: {e}"))?;
    if !status.success() {
        return Err(format!("git clone of {git_url} failed"));
    }

    let manifest = scratch.join("sheets-theme.json");
    let result = (|| {
        let contents = fs::read_to_string(&manifest)
            .map_err(|_| "repo has no sheets-theme.json at its root".to_string())?;
        let theme = parse_theme_manifest(&contents, git_url)?;

        let mut themes = load_user_themes()?;
        themes.retain(|t| t.id != theme.id);
        themes.push(theme.clone());
        save_user_themes(&themes)?;
        Ok(theme)
    })();

    let _ = fs::remove_dir_all(&scratch);
    result
}

pub(crate) fn slugify(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    // Kept byte-for-byte in sync with the example in the README's
    // "Theme repo format" section.
    const README_EXAMPLE: &str = r#"{
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
}"#;

    #[test]
    fn theme_manifest_matches_documented_format() {
        let theme = parse_theme_manifest(README_EXAMPLE, "https://example.com/some-theme.git").unwrap();
        assert_eq!(theme.id, "tokyo-night");
        assert_eq!(theme.name, "Tokyo Night");
        assert_eq!(theme.variant, ThemeVariant::Dark);
        assert_eq!(theme.source, ThemeSource::UserInstalled);
        assert_eq!(theme.git_url.as_deref(), Some("https://example.com/some-theme.git"));
        assert_eq!(theme.palette.background, "1a1b26");
        assert_eq!(theme.palette.selection_foreground, None);
        assert_eq!(theme.palette.ansi[15], "c0caf5");
    }
}
