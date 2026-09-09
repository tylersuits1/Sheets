use crate::apps::TerminalApp;
use crate::config_dir::sheets_data_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// A per-app pairing of "the theme I use during the day" and "the theme I
/// use at night" — separate from whichever theme is actually applied right
/// now. Setting one doesn't apply it; it just records the designation so
/// the status panel (and, later, an auto-switch feature) can show it.
#[derive(Serialize, Deserialize, Default)]
struct Assignment {
    day_theme_id: Option<String>,
    night_theme_id: Option<String>,
}

fn path_for(app: TerminalApp) -> Result<PathBuf, String> {
    Ok(sheets_data_dir()?.join("day_night").join(format!("{}.json", app.as_str())))
}

fn load(app: TerminalApp) -> Result<Assignment, String> {
    let path = path_for(app)?;
    if !path.exists() {
        return Ok(Assignment::default());
    }
    let contents = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&contents).map_err(|e| e.to_string())
}

fn save(app: TerminalApp, assignment: &Assignment) -> Result<(), String> {
    let path = path_for(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, serde_json::to_string(assignment).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

pub fn get_day_theme_id(app: TerminalApp) -> Result<Option<String>, String> {
    Ok(load(app)?.day_theme_id)
}

pub fn get_night_theme_id(app: TerminalApp) -> Result<Option<String>, String> {
    Ok(load(app)?.night_theme_id)
}

pub fn set_day_theme(app: TerminalApp, theme_id: String) -> Result<(), String> {
    let mut assignment = load(app)?;
    assignment.day_theme_id = Some(theme_id);
    save(app, &assignment)
}

pub fn set_night_theme(app: TerminalApp, theme_id: String) -> Result<(), String> {
    let mut assignment = load(app)?;
    assignment.night_theme_id = Some(theme_id);
    save(app, &assignment)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::ENV_LOCK;

    #[test]
    fn day_and_night_are_independent_and_persist_separately_per_app() {
        let _guard = ENV_LOCK.lock().unwrap();
        let scratch = std::env::temp_dir().join(format!("sheets-day-night-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&scratch);
        std::env::set_var("XDG_CONFIG_HOME", &scratch);

        assert_eq!(get_day_theme_id(TerminalApp::Ghostty).unwrap(), None);
        assert_eq!(get_night_theme_id(TerminalApp::Ghostty).unwrap(), None);

        set_day_theme(TerminalApp::Ghostty, "solarized-light".into()).unwrap();
        assert_eq!(get_day_theme_id(TerminalApp::Ghostty).unwrap(), Some("solarized-light".to_string()));
        assert_eq!(get_night_theme_id(TerminalApp::Ghostty).unwrap(), None, "setting day shouldn't touch night");

        set_night_theme(TerminalApp::Ghostty, "tokyo-night".into()).unwrap();
        assert_eq!(get_night_theme_id(TerminalApp::Ghostty).unwrap(), Some("tokyo-night".to_string()));
        assert_eq!(
            get_day_theme_id(TerminalApp::Ghostty).unwrap(),
            Some("solarized-light".to_string()),
            "setting night shouldn't clear day"
        );

        assert_eq!(get_day_theme_id(TerminalApp::Kitty).unwrap(), None, "assignments are per app");

        let _ = fs::remove_dir_all(&scratch);
    }
}
