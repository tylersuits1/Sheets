use crate::apps::TerminalApp;
use crate::config_dir::sheets_data_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// A user-picked config file location for an app, for when Sheets' XDG-based
/// auto-detection doesn't match where that app's config actually lives (a
/// custom dotfiles setup, a non-default install, etc). Takes priority over
/// the auto-detected default whenever one is set.
#[derive(Serialize, Deserialize, Default)]
struct Override {
    path: Option<String>,
}

fn path_for(app: TerminalApp) -> Result<PathBuf, String> {
    Ok(sheets_data_dir()?.join("config_overrides").join(format!("{}.json", app.as_str())))
}

fn load(app: TerminalApp) -> Result<Override, String> {
    let path = path_for(app)?;
    if !path.exists() {
        return Ok(Override::default());
    }
    let contents = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&contents).map_err(|e| e.to_string())
}

fn save(app: TerminalApp, override_: &Override) -> Result<(), String> {
    let path = path_for(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, serde_json::to_string(override_).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

/// The user-picked config path for `app`, if "Locate config file…" has ever
/// been used for it.
pub fn get(app: TerminalApp) -> Result<Option<PathBuf>, String> {
    Ok(load(app)?.path.map(PathBuf::from))
}

pub fn set(app: TerminalApp, config_path: &Path) -> Result<(), String> {
    save(app, &Override { path: Some(config_path.display().to_string()) })
}

pub fn clear(app: TerminalApp) -> Result<(), String> {
    save(app, &Override::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::ENV_LOCK;

    #[test]
    fn override_persists_and_clears_independently_per_app() {
        let _guard = ENV_LOCK.lock().unwrap();
        let scratch = std::env::temp_dir().join(format!("sheets-config-override-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&scratch);
        std::env::set_var("XDG_CONFIG_HOME", &scratch);

        assert_eq!(get(TerminalApp::Ghostty).unwrap(), None);

        set(TerminalApp::Ghostty, Path::new("/custom/path/config")).unwrap();
        assert_eq!(get(TerminalApp::Ghostty).unwrap(), Some(PathBuf::from("/custom/path/config")));
        assert_eq!(get(TerminalApp::Kitty).unwrap(), None, "overrides are per app");

        clear(TerminalApp::Ghostty).unwrap();
        assert_eq!(get(TerminalApp::Ghostty).unwrap(), None);

        let _ = fs::remove_dir_all(&scratch);
    }
}
