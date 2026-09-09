use crate::config_dir::sheets_data_dir;
use crate::theme::{Palette, Theme, ThemeSource, ThemeVariant};
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
    themes.extend(load_user_themes()?);
    Ok(themes)
}

pub fn get_theme(id: &str) -> Result<Theme, String> {
    list_themes()?
        .into_iter()
        .find(|t| t.id == id)
        .ok_or_else(|| format!("no theme with id \"{id}\""))
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

/// Clones `git_url` into a scratch directory and reads a `sheets-theme.json`
/// file (matching the `Theme` shape, minus `id`/`source`/`git_url`) from its
/// root. Repos in other formats (base16, iTerm color schemes, etc.) aren't
/// supported yet — that needs a converter, which is a separate piece of work.
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

        #[derive(serde::Deserialize)]
        struct ThemeManifest {
            name: String,
            variant: ThemeVariant,
            palette: Palette,
        }
        let parsed: ThemeManifest = serde_json::from_str(&contents).map_err(|e| e.to_string())?;

        let theme = Theme {
            id: slugify(&parsed.name),
            name: parsed.name,
            variant: parsed.variant,
            source: ThemeSource::UserInstalled,
            git_url: Some(git_url.to_string()),
            palette: parsed.palette,
        };

        let mut themes = load_user_themes()?;
        themes.retain(|t| t.id != theme.id);
        themes.push(theme.clone());
        save_user_themes(&themes)?;
        Ok(theme)
    })();

    let _ = fs::remove_dir_all(&scratch);
    result
}

fn slugify(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
